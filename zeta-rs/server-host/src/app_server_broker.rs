#[cfg(any(unix, windows))]
mod platform {
    use std::collections::BTreeMap;
    use std::fs;
    #[cfg(unix)]
    use std::fs::DirBuilder;
    use std::fs::File;
    use std::fs::OpenOptions;
    use std::io;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::io::Read;
    use std::io::Write;
    use std::net::Shutdown;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;
    use std::process::Stdio;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::Duration;
    use std::time::Instant;

    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(unix)]
    use std::os::unix::fs::DirBuilderExt;
    #[cfg(unix)]
    use std::os::unix::fs::FileTypeExt;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::os::unix::process::CommandExt;

    use serde::Deserialize;
    use serde::Serialize;
    use sha2::Digest;
    use sha2::Sha256;
    use zeta_app_server::AppServer;
    use zeta_app_server::LocalProductServicesConfig;
    use zeta_app_server::LocalProfileRuntime;
    use zeta_app_server_protocol::schema_hash;
    use zeta_uds::UnixListener;
    use zeta_uds::UnixStream;

    use crate::app_server::AppServerHostOptions;
    use crate::app_server::WorkspaceTrustSource;
    use crate::app_server::open_server_with_profile_runtime;

    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    const CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(50);
    const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
    const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(50);
    const IDLE_TIMEOUT_ENV: &str = "ZETA_LOCAL_APP_SERVER_IDLE_TIMEOUT_MILLIS";
    const MAX_LOG_BYTES: u64 = 1024 * 1024;
    const MAX_PRODUCT_SERVICES_IDENTITY_BYTES: u64 = 1024 * 1024;
    const MAX_CONNECTION_PRELUDE_BYTES: usize = 16 * 1024;
    const PROFILE_ROOT_ENV: &str = "ZETA_PROFILE_ROOT";
    const PRODUCT_SERVICES_ARGUMENT: &str = "--product-services";
    const STALE_START_LOCK_AGE: Duration = Duration::from_secs(15);
    const WORKSPACE_ROOT_ENV: &str = "ZETA_WORKSPACE_ROOT";
    const WORKSPACE_TRUST_SOURCE_ENV: &str = "ZETA_WORKSPACE_TRUST_SOURCE";

    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ConnectionPrelude {
        version: u32,
        workspace_root: Option<PathBuf>,
        workspace_trust_source: ConnectionWorkspaceTrustSource,
        product_services: Option<PathBuf>,
    }

    #[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    #[serde(rename_all = "camelCase")]
    enum ConnectionWorkspaceTrustSource {
        HostConfiguration,
        UserConfig,
    }

    impl ConnectionPrelude {
        fn from_options(options: &AppServerHostOptions) -> Self {
            Self {
                version: 1,
                workspace_root: options.workspace_root().map(Path::to_path_buf),
                workspace_trust_source: match options.workspace_trust_source() {
                    WorkspaceTrustSource::HostConfiguration => {
                        ConnectionWorkspaceTrustSource::HostConfiguration
                    }
                    WorkspaceTrustSource::UserConfig => ConnectionWorkspaceTrustSource::UserConfig,
                },
                product_services: options.product_services().map(Path::to_path_buf),
            }
        }

        fn trust_source(&self) -> WorkspaceTrustSource {
            match self.workspace_trust_source {
                ConnectionWorkspaceTrustSource::HostConfiguration => {
                    WorkspaceTrustSource::HostConfiguration
                }
                ConnectionWorkspaceTrustSource::UserConfig => WorkspaceTrustSource::UserConfig,
            }
        }
    }

    #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct WorkspaceRuntimeKey {
        workspace_root: Option<PathBuf>,
        workspace_trust_source: ConnectionWorkspaceTrustSource,
        product_services_identity: Option<[u8; 32]>,
    }

    struct ProfileAppServerRegistry {
        host: AppServerHostOptions,
        profile_runtime: Arc<LocalProfileRuntime>,
        servers: Mutex<BTreeMap<WorkspaceRuntimeKey, Arc<AppServer>>>,
    }

    impl ProfileAppServerRegistry {
        fn open(host: AppServerHostOptions) -> Result<Self, String> {
            let profile_runtime = Arc::new(
                LocalProfileRuntime::open(host.profile_root())
                    .map_err(|error| error.to_string())?,
            );
            Ok(Self {
                host,
                profile_runtime,
                servers: Mutex::new(BTreeMap::new()),
            })
        }

        fn server_for(&self, prelude: ConnectionPrelude) -> Result<Arc<AppServer>, String> {
            if prelude.version != 1 {
                return Err("unsupported local App Server connection prelude version".into());
            }
            let workspace_root = prelude
                .workspace_root
                .as_deref()
                .map(fs::canonicalize)
                .transpose()
                .map_err(io_error)?;
            let product_services_identity = product_services_identity(
                prelude.product_services.as_deref(),
                self.host.profile_root(),
            )?;
            let key = WorkspaceRuntimeKey {
                workspace_root: workspace_root.clone(),
                workspace_trust_source: prelude.workspace_trust_source,
                product_services_identity,
            };
            let mut servers = self
                .servers
                .lock()
                .map_err(|_| "local App Server Workspace registry lock poisoned".to_string())?;
            if let Some(server) = servers.get(&key) {
                return Ok(Arc::clone(server));
            }
            let host = AppServerHostOptions::new(
                self.host.profile_root(),
                workspace_root,
                prelude.trust_source(),
                prelude.product_services,
            );
            let server = Arc::new(open_server_with_profile_runtime(
                &host,
                Some(Arc::clone(&self.profile_runtime)),
            )?);
            servers.insert(key, Arc::clone(&server));
            Ok(server)
        }

        fn active_terminal_count(&self) -> usize {
            self.servers
                .lock()
                .map(|servers| {
                    servers
                        .values()
                        .map(|server| server.active_terminal_count())
                        .sum()
                })
                .unwrap_or(1)
        }
    }

    pub(super) fn connect(options: AppServerHostOptions) -> Result<(), String> {
        let endpoint = BrokerEndpoint::prepare(&options)?;
        let stream = connect_or_start(&endpoint, &options)?;
        proxy_stdio(stream, &options).map_err(|error| error.to_string())
    }

    pub(super) fn serve(options: AppServerHostOptions) -> Result<(), String> {
        let endpoint = BrokerEndpoint::prepare(&options)?;
        let listener = bind_listener(&endpoint)?;
        let _socket_cleanup = SocketCleanup(endpoint.socket.clone());
        let _ = fs::remove_dir(&endpoint.start_lock);
        listener.set_nonblocking(true).map_err(io_error)?;
        let registry = Arc::new(ProfileAppServerRegistry::open(options)?);
        let active_connections = Arc::new(AtomicUsize::new(0));
        let idle_timeout = configured_idle_timeout()?;
        let mut idle_since = None;
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false).map_err(io_error)?;
                    idle_since = None;
                    active_connections.fetch_add(1, Ordering::AcqRel);
                    let registry = Arc::clone(&registry);
                    let connection_counter = Arc::clone(&active_connections);
                    thread::Builder::new()
                        .name("zeta-local-app-server-connection".into())
                        .spawn(move || {
                            let _connection = ActiveConnection(connection_counter);
                            let reader = match stream.try_clone() {
                                Ok(reader) => reader,
                                Err(error) => {
                                    eprintln!("local App Server connection clone failed: {error}");
                                    return;
                                }
                            };
                            let mut reader = BufReader::new(reader);
                            let prelude = match read_connection_prelude(&mut reader) {
                                Ok(prelude) => prelude,
                                Err(error) => {
                                    eprintln!(
                                        "local App Server connection prelude failed: {error}"
                                    );
                                    return;
                                }
                            };
                            let server = match registry.server_for(prelude) {
                                Ok(server) => server,
                                Err(error) => {
                                    eprintln!("local App Server Workspace runtime failed: {error}");
                                    return;
                                }
                            };
                            if let Err(error) = server.serve_jsonl(reader, stream) {
                                eprintln!("local App Server connection failed: {error}");
                            }
                        })
                        .map_err(|error| {
                            active_connections.fetch_sub(1, Ordering::AcqRel);
                            error.to_string()
                        })?;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if active_connections.load(Ordering::Acquire) == 0
                        && registry.active_terminal_count() == 0
                    {
                        let idle_since = idle_since.get_or_insert_with(Instant::now);
                        if idle_since.elapsed() >= idle_timeout {
                            return Ok(());
                        }
                    } else {
                        idle_since = None;
                    }
                    thread::sleep(IDLE_POLL_INTERVAL);
                }
                Err(error) => return Err(io_error(error)),
            }
        }
    }

    struct ActiveConnection(Arc<AtomicUsize>);

    impl Drop for ActiveConnection {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::AcqRel);
        }
    }

    struct SocketCleanup(PathBuf);

    impl Drop for SocketCleanup {
        fn drop(&mut self) {
            if socket_path_exists(&self.0) {
                let _ = fs::remove_file(&self.0);
            }
        }
    }

    struct BrokerEndpoint {
        socket: PathBuf,
        start_lock: PathBuf,
        log: PathBuf,
    }

    impl BrokerEndpoint {
        fn prepare(options: &AppServerHostOptions) -> Result<Self, String> {
            fs::create_dir_all(options.profile_root()).map_err(io_error)?;
            let profile_root = fs::canonicalize(options.profile_root()).map_err(io_error)?;
            let runtime_root = runtime_root(&profile_root);
            ensure_private_runtime_root(&runtime_root)?;
            let identity = endpoint_identity(options, &profile_root, None)?;
            Ok(Self {
                socket: runtime_root.join(format!("{identity}.sock")),
                start_lock: runtime_root.join(format!("{identity}.starting")),
                log: runtime_root.join(format!("{identity}.log")),
            })
        }
    }

    pub(super) fn endpoint_identity(
        _options: &AppServerHostOptions,
        profile_root: &Path,
        _workspace_root: Option<&Path>,
    ) -> Result<String, String> {
        let mut digest = Sha256::new();
        update_path_digest(&mut digest, profile_root);
        digest.update(env!("CARGO_PKG_VERSION").as_bytes());
        digest.update([0]);
        digest.update(schema_hash().as_bytes());
        let identity = format!("{:x}", digest.finalize());
        Ok(identity[..32].into())
    }

    fn product_services_identity(
        path: Option<&Path>,
        profile_root: &Path,
    ) -> Result<Option<[u8; 32]>, String> {
        let Some(path) = path else {
            return Ok(None);
        };
        let metadata = fs::symlink_metadata(path).map_err(io_error)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAX_PRODUCT_SERVICES_IDENTITY_BYTES
        {
            return Err("Product services manifest is not a bounded regular file".into());
        }
        let services = LocalProductServicesConfig::load(path, profile_root)
            .map_err(|error| error.to_string())?;
        Ok(Some(*services.authority_identity()))
    }

    #[cfg(unix)]
    fn update_path_digest(digest: &mut Sha256, path: &Path) {
        digest.update(path.as_os_str().as_bytes());
        digest.update([0]);
    }

    #[cfg(windows)]
    fn update_path_digest(digest: &mut Sha256, path: &Path) {
        use std::os::windows::ffi::OsStrExt;

        for code_unit in path.as_os_str().encode_wide() {
            digest.update(code_unit.to_le_bytes());
        }
        digest.update([0]);
    }

    #[cfg(unix)]
    fn runtime_root(_profile_root: &Path) -> PathBuf {
        let effective_uid = rustix::process::geteuid().as_raw();
        PathBuf::from(format!("/tmp/zeta-local-app-server-{effective_uid}"))
    }

    #[cfg(windows)]
    fn runtime_root(profile_root: &Path) -> PathBuf {
        profile_root.join("run")
    }

    #[cfg(unix)]
    fn ensure_private_runtime_root(path: &Path) -> Result<(), String> {
        let effective_uid = rustix::process::geteuid().as_raw();
        match fs::symlink_metadata(path) {
            Ok(metadata) => validate_private_directory(path, &metadata, effective_uid),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                DirBuilder::new()
                    .mode(0o700)
                    .create(path)
                    .map_err(io_error)?;
                let metadata = fs::symlink_metadata(path).map_err(io_error)?;
                validate_private_directory(path, &metadata, effective_uid)
            }
            Err(error) => Err(io_error(error)),
        }
    }

    #[cfg(unix)]
    fn validate_private_directory(
        path: &Path,
        metadata: &fs::Metadata,
        effective_uid: u32,
    ) -> Result<(), String> {
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.uid() != effective_uid
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(format!(
                "Local App Server runtime directory is not private: {}",
                path.display()
            ));
        }
        Ok(())
    }

    #[cfg(windows)]
    fn ensure_private_runtime_root(path: &Path) -> Result<(), String> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => Ok(()),
            Ok(_) => Err(format!(
                "Local App Server runtime directory is invalid: {}",
                path.display()
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(path).map_err(io_error)
            }
            Err(error) => Err(io_error(error)),
        }
    }

    fn configured_idle_timeout() -> Result<Duration, String> {
        let Some(value) = std::env::var_os(IDLE_TIMEOUT_ENV) else {
            return Ok(DEFAULT_IDLE_TIMEOUT);
        };
        let value = value.to_string_lossy();
        let millis = value
            .parse::<u64>()
            .map_err(|_| format!("{IDLE_TIMEOUT_ENV} must be milliseconds"))?;
        if millis < IDLE_POLL_INTERVAL.as_millis() as u64 {
            return Err(format!(
                "{IDLE_TIMEOUT_ENV} must be at least {}",
                IDLE_POLL_INTERVAL.as_millis()
            ));
        }
        Ok(Duration::from_millis(millis))
    }

    fn connect_or_start(
        endpoint: &BrokerEndpoint,
        options: &AppServerHostOptions,
    ) -> Result<UnixStream, String> {
        if let Some(stream) = connect_existing(&endpoint.socket)? {
            return Ok(stream);
        }
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let mut owns_start_lock = false;
        while Instant::now() < deadline {
            if let Some(stream) = connect_existing(&endpoint.socket)? {
                if owns_start_lock {
                    let _ = fs::remove_dir(&endpoint.start_lock);
                }
                return Ok(stream);
            }
            if !owns_start_lock && acquire_start_lock(&endpoint.start_lock)? {
                owns_start_lock = true;
                if let Err(error) = spawn_daemon(endpoint, options) {
                    let _ = fs::remove_dir(&endpoint.start_lock);
                    return Err(error);
                }
            }
            thread::sleep(CONNECT_RETRY_INTERVAL);
        }
        if owns_start_lock {
            let _ = fs::remove_dir(&endpoint.start_lock);
        }
        Err(format!(
            "Local App Server daemon did not become ready; inspect {}",
            endpoint.log.display()
        ))
    }

    fn connect_existing(path: &Path) -> Result<Option<UnixStream>, String> {
        match UnixStream::connect(path) {
            Ok(stream) => Ok(Some(stream)),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(io_error(error)),
        }
    }

    fn acquire_start_lock(path: &Path) -> Result<bool, String> {
        match create_private_directory(path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(path).map_err(io_error)?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err("Local App Server start lock is not a real directory".into());
                }
                let stale = metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age >= STALE_START_LOCK_AGE);
                if stale {
                    fs::remove_dir(path).map_err(io_error)?;
                }
                Ok(false)
            }
            Err(error) => Err(io_error(error)),
        }
    }

    #[cfg(unix)]
    fn create_private_directory(path: &Path) -> io::Result<()> {
        DirBuilder::new().mode(0o700).create(path)
    }

    #[cfg(windows)]
    fn create_private_directory(path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn spawn_daemon(
        endpoint: &BrokerEndpoint,
        options: &AppServerHostOptions,
    ) -> Result<(), String> {
        let executable = std::env::current_exe().map_err(io_error)?;
        let log = open_log(&endpoint.log)?;
        let error_log = log.try_clone().map_err(io_error)?;
        let mut command = Command::new(executable);
        command
            .args(["app-server", "daemon"])
            .env(PROFILE_ROOT_ENV, options.profile_root())
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(error_log));
        match options.workspace_root() {
            Some(workspace_root) => {
                command.env(WORKSPACE_ROOT_ENV, workspace_root);
            }
            None => {
                command.env_remove(WORKSPACE_ROOT_ENV);
            }
        }
        command.env(
            WORKSPACE_TRUST_SOURCE_ENV,
            match options.workspace_trust_source() {
                WorkspaceTrustSource::HostConfiguration => "hostConfiguration",
                WorkspaceTrustSource::UserConfig => "userConfig",
            },
        );
        if let Some(path) = options.product_services() {
            command.arg(PRODUCT_SERVICES_ARGUMENT).arg(path);
        }
        detach_command(&mut command);
        command.spawn().map_err(io_error)?;
        Ok(())
    }

    #[cfg(unix)]
    fn detach_command(command: &mut Command) {
        command.process_group(0);
    }

    #[cfg(windows)]
    fn detach_command(_command: &mut Command) {}

    #[cfg(unix)]
    fn open_log(path: &Path) -> Result<File, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(io_error)?;
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err("Local App Server log is not a private regular file".into());
        }
        truncate_log(&file, &metadata)?;
        Ok(file)
    }

    #[cfg(windows)]
    fn open_log(path: &Path) -> Result<File, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(io_error)?;
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_file() {
            return Err("Local App Server log is not a regular file".into());
        }
        truncate_log(&file, &metadata)?;
        Ok(file)
    }

    fn truncate_log(file: &File, metadata: &fs::Metadata) -> Result<(), String> {
        if metadata.len() > MAX_LOG_BYTES {
            file.set_len(0).map_err(io_error)?;
        }
        Ok(())
    }

    fn bind_listener(endpoint: &BrokerEndpoint) -> Result<UnixListener, String> {
        match connect_existing(&endpoint.socket)? {
            Some(_) => return Err("Local App Server daemon is already running".into()),
            None => remove_stale_socket(&endpoint.socket)?,
        }
        let listener = UnixListener::bind(&endpoint.socket).map_err(io_error)?;
        set_socket_permissions(&endpoint.socket)?;
        Ok(listener)
    }

    #[cfg(unix)]
    fn set_socket_permissions(path: &Path) -> Result<(), String> {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_error)
    }

    #[cfg(windows)]
    fn set_socket_permissions(_path: &Path) -> Result<(), String> {
        Ok(())
    }

    #[cfg(unix)]
    fn remove_stale_socket(path: &Path) -> Result<(), String> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_socket() => {
                fs::remove_file(path).map_err(io_error)
            }
            Ok(_) => Err("Local App Server endpoint is not a Unix socket".into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }

    #[cfg(windows)]
    fn remove_stale_socket(path: &Path) -> Result<(), String> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_file() => {
                fs::remove_file(path).map_err(io_error)
            }
            Ok(_) => Err("Local App Server endpoint is invalid".into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }

    #[cfg(unix)]
    fn socket_path_exists(path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_socket())
    }

    #[cfg(windows)]
    fn socket_path_exists(path: &Path) -> bool {
        fs::symlink_metadata(path)
            .is_ok_and(|metadata| !metadata.file_type().is_symlink() && metadata.is_file())
    }

    fn read_connection_prelude(
        reader: &mut BufReader<UnixStream>,
    ) -> Result<ConnectionPrelude, String> {
        let mut line = String::new();
        let read = reader
            .by_ref()
            .take((MAX_CONNECTION_PRELUDE_BYTES + 1) as u64)
            .read_line(&mut line)
            .map_err(io_error)?;
        if read == 0 || read > MAX_CONNECTION_PRELUDE_BYTES || !line.ends_with('\n') {
            return Err("local App Server connection prelude is missing or too large".into());
        }
        serde_json::from_str(&line).map_err(|error| error.to_string())
    }

    fn proxy_stdio(mut stream: UnixStream, options: &AppServerHostOptions) -> io::Result<()> {
        serde_json::to_writer(&mut stream, &ConnectionPrelude::from_options(options))?;
        stream.write_all(b"\n")?;
        stream.flush()?;
        let mut socket_writer = stream.try_clone()?;
        let input = thread::Builder::new()
            .name("zeta-local-app-server-stdin".into())
            .spawn(move || {
                let copied = io::copy(&mut io::stdin().lock(), &mut socket_writer);
                let _ = socket_writer.shutdown(Shutdown::Write);
                copied
            })?;
        let mut output = io::stdout().lock();
        io::copy(&mut BufReader::new(stream), &mut output)?;
        output.flush()?;
        input
            .join()
            .map_err(|_| io::Error::other("Local App Server stdin proxy panicked"))??;
        Ok(())
    }

    fn io_error(error: io::Error) -> String {
        error.to_string()
    }
}

use crate::app_server::AppServerHostOptions;

#[cfg(any(unix, windows))]
pub(super) fn connect(options: AppServerHostOptions) -> Result<(), String> {
    platform::connect(options)
}

#[cfg(any(unix, windows))]
pub(super) fn serve(options: AppServerHostOptions) -> Result<(), String> {
    platform::serve(options)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn connect(_options: AppServerHostOptions) -> Result<(), String> {
    Err("Local App Server daemon requires Unix-domain socket support".into())
}

#[cfg(not(any(unix, windows)))]
pub(super) fn serve(_options: AppServerHostOptions) -> Result<(), String> {
    Err("Local App Server daemon requires Unix-domain socket support".into())
}

#[cfg(test)]
#[path = "app_server_broker_tests.rs"]
mod tests;
