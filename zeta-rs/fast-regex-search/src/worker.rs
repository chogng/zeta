use crate::FastRegexError;
use crate::FastRegexQuery;
use crate::FastRegexSearch;
use crate::FastRegexSearchLimits;
use crate::FastRegexSearchResult;
use crate::FastRegexSearchSnapshot;
use crate::FastRegexSearchStorage;
use crate::FastRegexUpdateOutcome;
use serde::Deserialize;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use zeta_uds::UnixListener;
use zeta_uds::UnixStream;
use zeta_workspace::WorkspaceRoot;

const PROTOCOL_VERSION: u16 = 1;
const MAX_REQUEST_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const WORKER_ENDPOINT_ENV: &str = "ZETA_FAST_REGEX_WORKER_ENDPOINT";
const WORKER_LIMITS_ENV: &str = "ZETA_FAST_REGEX_WORKER_LIMITS";
const WORKER_ROOT_ENV: &str = "ZETA_FAST_REGEX_WORKER_ROOT";
const WORKER_STORAGE_ENV: &str = "ZETA_FAST_REGEX_WORKER_STORAGE";
static ENDPOINT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Describes how the host executable enters its private Fast Regex worker role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FastRegexWorkerCommand {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl FastRegexWorkerCommand {
    pub fn new(
        executable: impl Into<PathBuf>,
        arguments: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        Self {
            executable: executable.into(),
            arguments: arguments.into_iter().map(Into::into).collect(),
        }
    }
}

/// Owns one long-lived Fast Regex worker and exchanges bounded request/response messages over UDS.
pub struct FastRegexWorkerClient {
    command: FastRegexWorkerCommand,
    root: PathBuf,
    storage: PathBuf,
    limits: FastRegexSearchLimits,
    endpoint_directory: PathBuf,
    endpoint: PathBuf,
    child: Mutex<Option<Child>>,
    mutation: Mutex<()>,
    requests: RwLock<()>,
}

impl FastRegexWorkerClient {
    pub fn open(
        command: FastRegexWorkerCommand,
        root: &WorkspaceRoot,
        storage: impl Into<PathBuf>,
        limits: FastRegexSearchLimits,
    ) -> Result<Self, FastRegexError> {
        let endpoint_directory = create_endpoint_directory()?;
        let endpoint = endpoint_directory.join("worker.sock");
        let client = Self {
            command,
            root: root.requested_path().to_path_buf(),
            storage: storage.into(),
            limits,
            endpoint_directory,
            endpoint,
            child: Mutex::new(None),
            mutation: Mutex::new(()),
            requests: RwLock::new(()),
        };
        client.start_worker()?;
        Ok(client)
    }

    pub fn snapshot(&self) -> Result<FastRegexSearchSnapshot, FastRegexError> {
        match self.request(WorkerRequest::Snapshot)? {
            WorkerValue::Snapshot(snapshot) => Ok(snapshot),
            _ => Err(protocol_error(
                "worker returned the wrong snapshot response",
            )),
        }
    }

    pub fn rebuild(&self) -> Result<FastRegexSearchSnapshot, FastRegexError> {
        let _mutation = self
            .mutation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match self.request(WorkerRequest::Rebuild)? {
            WorkerValue::Snapshot(snapshot) => {
                self.restart_worker()?;
                Ok(snapshot)
            }
            _ => Err(protocol_error("worker returned the wrong rebuild response")),
        }
    }

    pub fn refresh_observed_paths(
        &self,
        paths: &[PathBuf],
    ) -> Result<FastRegexUpdateOutcome, FastRegexError> {
        let _mutation = self
            .mutation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match self.request(WorkerRequest::RefreshObservedPaths {
            paths: paths.to_vec(),
        })? {
            WorkerValue::Update(outcome) => {
                if matches!(outcome, FastRegexUpdateOutcome::Rebuilt(_)) {
                    self.restart_worker()?;
                }
                Ok(outcome)
            }
            _ => Err(protocol_error("worker returned the wrong refresh response")),
        }
    }

    pub fn search(&self, query: &FastRegexQuery) -> Result<FastRegexSearchResult, FastRegexError> {
        match self.request(WorkerRequest::Search {
            query: query.clone(),
        })? {
            WorkerValue::Search(result) => Ok(result),
            _ => Err(protocol_error("worker returned the wrong search response")),
        }
    }

    /// Returns the current worker process identifier for benchmarks and diagnostics.
    pub fn process_id(&self) -> Option<u32> {
        self.child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(Child::id)
    }

    fn request(&self, request: WorkerRequest) -> Result<WorkerValue, FastRegexError> {
        let _request = self
            .requests
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.request_without_gate(request)
    }

    fn request_without_gate(&self, request: WorkerRequest) -> Result<WorkerValue, FastRegexError> {
        let mut stream = match UnixStream::connect(&self.endpoint) {
            Ok(stream) => stream,
            Err(_) => {
                self.start_worker()?;
                UnixStream::connect(&self.endpoint).map_err(|error| worker_io("connect", error))?
            }
        };
        let envelope = RequestEnvelope {
            version: PROTOCOL_VERSION,
            request,
        };
        serde_json::to_writer(&mut stream, &envelope)
            .map_err(|error| protocol_error(error.to_string()))?;
        stream
            .write_all(b"\n")
            .and_then(|_| stream.flush())
            .map_err(|error| worker_io("write request", error))?;

        let mut response = Vec::new();
        BufReader::new(stream)
            .take(MAX_RESPONSE_BYTES as u64 + 1)
            .read_until(b'\n', &mut response)
            .map_err(|error| worker_io("read response", error))?;
        if response.len() > MAX_RESPONSE_BYTES || !response.ends_with(b"\n") {
            return Err(protocol_error(format!(
                "worker response ended without a complete frame ({} bytes)",
                response.len()
            )));
        }
        let response: ResponseEnvelope =
            serde_json::from_slice(&response).map_err(|error| protocol_error(error.to_string()))?;
        if response.version != PROTOCOL_VERSION {
            return Err(protocol_error("worker protocol version mismatch"));
        }
        response.result.map_err(WireError::into_error)
    }

    fn start_worker(&self) -> Result<(), FastRegexError> {
        let mut child = self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if child
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_none())
            && UnixStream::connect(&self.endpoint).is_ok()
        {
            return Ok(());
        }
        if let Some(mut previous) = child.take() {
            let _ = previous.kill();
            let _ = previous.wait();
        }
        remove_socket_if_present(&self.endpoint)
            .map_err(|error| worker_io("remove socket", error))?;
        let limits = serde_json::to_string(&self.limits)
            .map_err(|error| protocol_error(error.to_string()))?;
        let spawned = Command::new(&self.command.executable)
            .args(&self.command.arguments)
            .env(WORKER_ENDPOINT_ENV, &self.endpoint)
            .env(WORKER_ROOT_ENV, &self.root)
            .env(WORKER_STORAGE_ENV, &self.storage)
            .env(WORKER_LIMITS_ENV, limits)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| worker_io("spawn", error))?;
        *child = Some(spawned);

        let started = Instant::now();
        loop {
            if UnixStream::connect(&self.endpoint).is_ok() {
                return Ok(());
            }
            if let Some(status) = child
                .as_mut()
                .and_then(|child| child.try_wait().ok().flatten())
            {
                return Err(protocol_error(format!(
                    "worker exited during startup with {status}"
                )));
            }
            if started.elapsed() >= STARTUP_TIMEOUT {
                if let Some(mut timed_out) = child.take() {
                    let _ = timed_out.kill();
                    let _ = timed_out.wait();
                }
                return Err(protocol_error("worker startup timed out"));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn restart_worker(&self) -> Result<(), FastRegexError> {
        let _requests = self
            .requests
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = self.request_without_gate(WorkerRequest::Shutdown);
        let mut child = self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(mut previous) = child.take() {
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                if previous.try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            if previous.try_wait().ok().flatten().is_none() {
                let _ = previous.kill();
            }
            let _ = previous.wait();
        }
        drop(child);
        remove_socket_if_present(&self.endpoint)
            .map_err(|error| worker_io("remove socket", error))?;
        self.start_worker()
    }
}

impl Drop for FastRegexWorkerClient {
    fn drop(&mut self) {
        if self.child.get_mut().ok().and_then(Option::as_mut).is_none() {
            let _ = remove_endpoint_directory(&self.endpoint, &self.endpoint_directory);
            return;
        }
        let _ = self.request(WorkerRequest::Shutdown);
        if let Ok(Some(child)) = self.child.get_mut() {
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                if child.try_wait().ok().flatten().is_some() {
                    let _ = remove_endpoint_directory(&self.endpoint, &self.endpoint_directory);
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = remove_endpoint_directory(&self.endpoint, &self.endpoint_directory);
    }
}

/// Runs the private worker role configured by [`FastRegexWorkerClient`].
pub fn serve_worker_from_environment() -> Result<(), FastRegexError> {
    let endpoint = required_path_environment(WORKER_ENDPOINT_ENV)?;
    let root = required_path_environment(WORKER_ROOT_ENV)?;
    let storage = required_path_environment(WORKER_STORAGE_ENV)?;
    let limits = std::env::var(WORKER_LIMITS_ENV)
        .map_err(|_| protocol_error("worker limits are missing"))
        .and_then(|value| {
            serde_json::from_str(&value).map_err(|error| protocol_error(error.to_string()))
        })?;
    let root = WorkspaceRoot::open(root).map_err(|error| protocol_error(error.to_string()))?;
    let search = Arc::new(FastRegexSearch::open(
        root,
        FastRegexSearchStorage::Persistent(storage),
        limits,
    )?);
    remove_socket_if_present(&endpoint).map_err(|error| worker_io("remove socket", error))?;
    let listener = UnixListener::bind(&endpoint).map_err(|error| worker_io("bind", error))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| worker_io("configure listener", error))?;
    let stopping = Arc::new(AtomicBool::new(false));
    while !stopping.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _address)) => {
                stream
                    .set_nonblocking(false)
                    .map_err(|error| worker_io("configure connection", error))?;
                let search = Arc::clone(&search);
                let stopping = Arc::clone(&stopping);
                thread::spawn(move || handle_connection(stream, &search, &stopping));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(worker_io("accept", error)),
        }
    }
    remove_socket_if_present(&endpoint).map_err(|error| worker_io("remove socket", error))
}

fn handle_connection(mut stream: UnixStream, search: &FastRegexSearch, stopping: &AtomicBool) {
    let Some(request) = read_request(&stream).transpose() else {
        return;
    };
    let result = request.and_then(|request| match request {
        WorkerRequest::Snapshot => Ok(WorkerValue::Snapshot(search.snapshot())),
        WorkerRequest::Rebuild => search.rebuild().map(WorkerValue::Snapshot),
        WorkerRequest::RefreshObservedPaths { paths } => search
            .refresh_observed_paths(&paths)
            .map(WorkerValue::Update),
        WorkerRequest::Search { query } => search.search(&query).map(WorkerValue::Search),
        WorkerRequest::Shutdown => {
            stopping.store(true, Ordering::Release);
            Ok(WorkerValue::Shutdown)
        }
    });
    let response = ResponseEnvelope {
        version: PROTOCOL_VERSION,
        result: result.map_err(WireError::from_error),
    };
    let Ok(mut bytes) = serde_json::to_vec(&response) else {
        return;
    };
    if bytes.len() >= MAX_RESPONSE_BYTES {
        bytes = serde_json::to_vec(&ResponseEnvelope {
            version: PROTOCOL_VERSION,
            result: Err(WireError {
                kind: WireErrorKind::Other,
                path: None,
                message: "serialized worker response exceeds the protocol limit".to_owned(),
            }),
        })
        .unwrap_or_default();
    }
    bytes.push(b'\n');
    let _ = stream.write_all(&bytes).and_then(|_| stream.flush());
}

fn read_request(stream: &UnixStream) -> Result<Option<WorkerRequest>, FastRegexError> {
    let mut request = Vec::new();
    BufReader::new(stream)
        .take(MAX_REQUEST_BYTES + 1)
        .read_until(b'\n', &mut request)
        .map_err(|error| worker_io("read request", error))?;
    if request.is_empty() {
        return Ok(None);
    }
    if request.len() > MAX_REQUEST_BYTES as usize || !request.ends_with(b"\n") {
        return Err(protocol_error("worker request exceeds the protocol limit"));
    }
    let request: RequestEnvelope =
        serde_json::from_slice(&request).map_err(|error| protocol_error(error.to_string()))?;
    if request.version != PROTOCOL_VERSION {
        return Err(protocol_error("worker protocol version mismatch"));
    }
    Ok(Some(request.request))
}

#[derive(Deserialize, Serialize)]
struct RequestEnvelope {
    version: u16,
    request: WorkerRequest,
}

#[derive(Deserialize, Serialize)]
enum WorkerRequest {
    Snapshot,
    Rebuild,
    RefreshObservedPaths {
        #[serde(with = "serde_paths")]
        paths: Vec<PathBuf>,
    },
    Search {
        query: FastRegexQuery,
    },
    Shutdown,
}

#[derive(Deserialize, Serialize)]
struct ResponseEnvelope {
    version: u16,
    result: Result<WorkerValue, WireError>,
}

#[derive(Deserialize, Serialize)]
enum WorkerValue {
    Snapshot(FastRegexSearchSnapshot),
    Update(FastRegexUpdateOutcome),
    Search(FastRegexSearchResult),
    Shutdown,
}

#[derive(Deserialize, Serialize)]
struct WireError {
    kind: WireErrorKind,
    #[serde(with = "serde_optional_path")]
    path: Option<PathBuf>,
    message: String,
}

#[derive(Deserialize, Serialize)]
enum WireErrorKind {
    NotReady,
    StaleSource,
    Other,
}

impl WireError {
    fn from_error(error: FastRegexError) -> Self {
        match error {
            FastRegexError::NotReady => Self {
                kind: WireErrorKind::NotReady,
                path: None,
                message: error.to_string(),
            },
            FastRegexError::StaleSource(path) => Self {
                kind: WireErrorKind::StaleSource,
                path: Some(path.clone()),
                message: FastRegexError::StaleSource(path).to_string(),
            },
            error => Self {
                kind: WireErrorKind::Other,
                path: None,
                message: error.to_string(),
            },
        }
    }

    fn into_error(self) -> FastRegexError {
        match self.kind {
            WireErrorKind::NotReady => FastRegexError::NotReady,
            WireErrorKind::StaleSource => self
                .path
                .map(FastRegexError::StaleSource)
                .unwrap_or_else(|| FastRegexError::Worker(self.message)),
            WireErrorKind::Other => FastRegexError::Worker(self.message),
        }
    }
}

fn required_path_environment(name: &str) -> Result<PathBuf, FastRegexError> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| protocol_error(format!("{name} is missing")))
}

fn create_endpoint_directory() -> Result<PathBuf, FastRegexError> {
    let root = std::env::temp_dir();
    for _ in 0..100 {
        let sequence = ENDPOINT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = root.join(format!("zeta-fast-regex-{}-{sequence}", std::process::id()));
        match fs::create_dir(&directory) {
            Ok(()) => {
                set_private_directory_permissions(&directory)?;
                return Ok(directory);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(worker_io("create endpoint directory", error)),
        }
    }
    Err(protocol_error("could not allocate a worker endpoint"))
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), FastRegexError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| worker_io("secure endpoint directory", error))
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), FastRegexError> {
    Ok(())
}

fn remove_endpoint_directory(endpoint: &Path, directory: &Path) -> io::Result<()> {
    remove_socket_if_present(endpoint)?;
    match fs::remove_dir(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_socket_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn worker_io(operation: &str, error: io::Error) -> FastRegexError {
    FastRegexError::Worker(format!("{operation}: {error}"))
}

fn protocol_error(message: impl Into<String>) -> FastRegexError {
    FastRegexError::Worker(message.into())
}

pub(crate) mod serde_path {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;

    #[cfg(unix)]
    pub fn serialize<S>(path: &Path, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::os::unix::ffi::OsStrExt;

        path.as_os_str().as_bytes().serialize(serializer)
    }

    #[cfg(unix)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::os::unix::ffi::OsStringExt;

        Vec::<u8>::deserialize(deserializer).map(|bytes| OsString::from_vec(bytes).into())
    }

    #[cfg(windows)]
    pub fn serialize<S>(path: &Path, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::os::windows::ffi::OsStrExt;

        path.as_os_str()
            .encode_wide()
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    #[cfg(windows)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::os::windows::ffi::OsStringExt;

        Vec::<u16>::deserialize(deserializer).map(|units| OsString::from_wide(&units).into())
    }
}

mod serde_paths {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[cfg(unix)]
    pub fn serialize<S>(paths: &[PathBuf], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::os::unix::ffi::OsStrExt;

        paths
            .iter()
            .map(|path| path.as_os_str().as_bytes())
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    #[cfg(unix)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::os::unix::ffi::OsStringExt;

        Vec::<Vec<u8>>::deserialize(deserializer).map(|paths| {
            paths
                .into_iter()
                .map(|path| PathBuf::from(OsString::from_vec(path)))
                .collect()
        })
    }

    #[cfg(windows)]
    pub fn serialize<S>(paths: &[PathBuf], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::os::windows::ffi::OsStrExt;

        paths
            .iter()
            .map(|path| path.as_os_str().encode_wide().collect::<Vec<_>>())
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    #[cfg(windows)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::os::windows::ffi::OsStringExt;

        Vec::<Vec<u16>>::deserialize(deserializer).map(|paths| {
            paths
                .into_iter()
                .map(|path| PathBuf::from(OsString::from_wide(&path)))
                .collect()
        })
    }
}

mod serde_optional_path {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[cfg(unix)]
    pub fn serialize<S>(path: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::os::unix::ffi::OsStrExt;

        path.as_ref()
            .map(|path| path.as_os_str().as_bytes())
            .serialize(serializer)
    }

    #[cfg(unix)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::os::unix::ffi::OsStringExt;

        Option::<Vec<u8>>::deserialize(deserializer)
            .map(|path| path.map(|path| PathBuf::from(OsString::from_vec(path))))
    }

    #[cfg(windows)]
    pub fn serialize<S>(path: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::os::windows::ffi::OsStrExt;

        path.as_ref()
            .map(|path| path.as_os_str().encode_wide().collect::<Vec<_>>())
            .serialize(serializer)
    }

    #[cfg(windows)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::os::windows::ffi::OsStringExt;

        Option::<Vec<u16>>::deserialize(deserializer)
            .map(|path| path.map(|path| PathBuf::from(OsString::from_wide(&path))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FastRegexCaseSensitivity;
    use crate::FastRegexPattern;
    use std::fs;

    #[test]
    fn worker_process_entrypoint() {
        if std::env::var_os(WORKER_ENDPOINT_ENV).is_none() {
            return;
        }
        serve_worker_from_environment().expect("serve worker");
    }

    #[test]
    fn worker_owns_rebuild_refresh_and_search() {
        let workspace = tempfile::tempdir().expect("workspace");
        let storage = tempfile::tempdir().expect("storage");
        fs::write(workspace.path().join("alpha.txt"), "worker needle\n").expect("source");
        #[cfg(target_os = "linux")]
        let non_utf8_path = {
            use std::os::unix::ffi::OsStringExt;

            let path = PathBuf::from(OsString::from_vec(b"non-utf8-\xff.txt".to_vec()));
            fs::write(workspace.path().join(&path), "encoded path marker\n")
                .expect("non-UTF-8 source");
            path
        };
        let root = WorkspaceRoot::open(workspace.path()).expect("root");
        let command = FastRegexWorkerCommand::new(
            std::env::current_exe().expect("test executable"),
            [
                OsString::from("--exact"),
                OsString::from("worker::tests::worker_process_entrypoint"),
                OsString::from("--nocapture"),
            ],
        );
        let client = Arc::new(
            FastRegexWorkerClient::open(
                command,
                &root,
                storage.path(),
                FastRegexSearchLimits::default(),
            )
            .expect("client"),
        );

        assert_eq!(client.snapshot().unwrap().generation, 0);
        let rebuilt = client.rebuild().expect("rebuild");
        assert_eq!(rebuilt.generation, 1);
        let result = client
            .search(&FastRegexQuery {
                query: "needle".to_owned(),
                pattern: FastRegexPattern::Literal,
                case_sensitivity: FastRegexCaseSensitivity::Sensitive,
                scope: PathBuf::new(),
                include_patterns: Vec::new(),
                exclude_patterns: Vec::new(),
                max_results: 10,
            })
            .expect("search");
        assert_eq!(result.matches.len(), 1);
        let searches = (0..8)
            .map(|_| {
                let client = Arc::clone(&client);
                thread::spawn(move || {
                    client.search(&FastRegexQuery {
                        query: "needle".to_owned(),
                        pattern: FastRegexPattern::Literal,
                        case_sensitivity: FastRegexCaseSensitivity::Sensitive,
                        scope: PathBuf::new(),
                        include_patterns: Vec::new(),
                        exclude_patterns: Vec::new(),
                        max_results: 10,
                    })
                })
            })
            .collect::<Vec<_>>();
        for search in searches {
            assert_eq!(
                search.join().expect("search thread").unwrap().matches.len(),
                1
            );
        }

        #[cfg(target_os = "linux")]
        {
            let encoded = client
                .search(&FastRegexQuery {
                    query: "encoded path marker".to_owned(),
                    pattern: FastRegexPattern::Literal,
                    case_sensitivity: FastRegexCaseSensitivity::Sensitive,
                    scope: PathBuf::new(),
                    include_patterns: Vec::new(),
                    exclude_patterns: Vec::new(),
                    max_results: 10,
                })
                .expect("search non-UTF-8 path");
            assert_eq!(encoded.matches[0].path, non_utf8_path);
        }

        fs::write(workspace.path().join("alpha.txt"), "changed value\n").expect("change");
        let outcome = client
            .refresh_observed_paths(&[workspace.path().join("alpha.txt")])
            .expect("refresh");
        assert!(matches!(outcome, FastRegexUpdateOutcome::Published(_)));
    }
}
