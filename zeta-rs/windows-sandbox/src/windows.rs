mod account;
mod attribution;
mod audit;
mod desktop;
mod filesystem;
mod job;
mod network;
mod process;
mod proxy;
mod runtime;
mod win;

use account::NetworkMode;
use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
use windows_sys::Win32::Foundation::WAIT_TIMEOUT;
use windows_sys::Win32::Security::DACL_SECURITY_INFORMATION;
use windows_sys::Win32::Security::PROTECTED_DACL_SECURITY_INFORMATION;
use windows_sys::Win32::Security::SetFileSecurityW;
use windows_sys::Win32::System::Threading::GetExitCodeProcess;
use windows_sys::Win32::System::Threading::WaitForSingleObject;
use wxc_common::host_changes::HostAclScope;
use wxc_common::models::ContainerPolicy;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::HostAclChanges;
use zeta_sandboxing::NetworkAccess;
use zeta_sandboxing::PreparedCommand;
use zeta_sandboxing::ProcessHandle;
use zeta_sandboxing::SandboxCommand;
use zeta_sandboxing::SandboxDirAccess;
use zeta_sandboxing::SandboxError;
use zeta_sandboxing::SandboxKind;
use zeta_sandboxing::SandboxLaunch;
use zeta_sandboxing::SandboxPolicy;
use zeta_sandboxing::SandboxProcess;
use zeta_sandboxing::SandboxProcessExitStatus;
use zeta_sandboxing::SandboxScope;

struct Execution {
    runner_hash: String,
    files: ContainerPolicy,
    host_acl_scope: Option<HostAclScope>,
    acl_changes: HostAclChanges,
    command: String,
    working_directory: String,
    env: Vec<String>,
    mode: NetworkMode,
    proxy_port: Option<u16>,
}

fn unavailable(error: impl ToString) -> SandboxError {
    SandboxError::BackendUnavailable {
        backend: SandboxKind::Restricted,
        message: error.to_string(),
    }
}

pub(super) fn prepare(
    installation: &zeta_install_context::InstallContext,
    command: &SandboxCommand,
    policy: SandboxPolicy,
    scope: &SandboxScope,
) -> Result<PreparedCommand, SandboxError> {
    if !policy.requires_platform_sandbox() {
        if !scope.is_single_unhidden() {
            return Err(SandboxError::InvalidScope(
                "unrestricted execution cannot hide directories".into(),
            ));
        }
        return Ok(PreparedCommand::unrestricted(command));
    }
    if policy.file_system() == FileSystemAccess::FullAccess {
        return Err(SandboxError::UnsupportedPolicy(
            "Windows account execution requires a restricted filesystem policy".into(),
        ));
    }
    if policy.file_system_isolation() != zeta_sandboxing::FileSystemIsolation::WindowsAccount {
        return Err(SandboxError::UnsupportedPolicy(
            "Windows account isolation requires explicit acceptance of bounded ACL auditing; it cannot satisfy strict host filesystem isolation".into(),
        ));
    }
    if policy.host_acl_changes() == HostAclChanges::Denied {
        return Err(SandboxError::UnsupportedPolicy(
            "Windows account execution requires scoped filesystem ACL authorization".into(),
        ));
    }
    let mut files = ContainerPolicy::default();
    for grant in scope.grants() {
        let root = grant.dir().canonical_path();
        let text = root
            .to_str()
            .ok_or_else(|| unavailable("filesystem paths must be Unicode"))?
            .to_owned();
        if policy.file_system() == FileSystemAccess::DirectoryWrite
            && grant.access() == SandboxDirAccess::ReadWrite
        {
            files.readwrite_paths.push(text);
            for name in zeta_sandboxing::PROTECTED_DIR_METADATA_NAMES {
                let path = root.join(name);
                match std::fs::symlink_metadata(&path) {
                    Ok(_) => files.readonly_paths.push(
                        path.to_str()
                            .ok_or_else(|| unavailable("filesystem paths must be Unicode"))?
                            .into(),
                    ),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(unavailable(error)),
                }
            }
        } else {
            files.readonly_paths.push(text);
        }
    }
    let authority = HostAclScope::new(
        scope
            .grants()
            .iter()
            .map(|grant| grant.dir().canonical_path().to_owned())
            .chain(
                scope
                    .hidden_dirs()
                    .iter()
                    .map(|dir| dir.canonical_path().to_owned()),
            ),
    )
    .map_err(unavailable)?;
    for dir in scope.hidden_dirs() {
        files.denied_paths.push(
            dir.canonical_path()
                .to_str()
                .ok_or_else(|| unavailable("filesystem paths must be Unicode"))?
                .into(),
        );
    }
    let argv = std::iter::once(command.program())
        .chain(command.arguments().iter().map(|arg| arg.as_os_str()))
        .map(|arg| {
            arg.to_str()
                .filter(|value| !value.contains('\0'))
                .map(str::to_owned)
                .ok_or_else(|| unavailable("command arguments must be Unicode without NUL"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let command_line = wxc_common::cmdline::cmdline_from_argv_for_context(
        &argv,
        wxc_common::cmdline::CommandLineContext::WindowsCreateProcess,
    )
    .map_err(unavailable)?;
    let mode = match policy.network() {
        NetworkAccess::Denied => NetworkMode::Denied,
        NetworkAccess::Allowed => NetworkMode::Allowed,
        NetworkAccess::Managed => NetworkMode::Managed,
    };
    let proxy_port = if mode == NetworkMode::Managed {
        let ports = command
            .network_proxy()
            .ok_or_else(|| unavailable("managed execution requires an execution-owned proxy"))?
            .ports();
        if ports[0] != ports[1] {
            return Err(SandboxError::UnsupportedPolicy(
                "Windows account execution requires a shared proxy endpoint".into(),
            ));
        }
        Some(ports[0])
    } else {
        None
    };
    let candidates =
        installation.executable_candidates(zeta_install_context::ManagedExecutable::WindowsSandbox);
    let helper = match candidates {
        zeta_install_context::ExecutableCandidates::ExplicitOverride(candidate) => {
            candidate.path().to_owned()
        }
        zeta_install_context::ExecutableCandidates::SearchPaths(paths) => paths
            .into_iter()
            .find(|path| path.is_file())
            .ok_or_else(|| unavailable("the product's Windows sandbox executable is missing"))?,
    };
    let _helper_pin = win::pin_executable(&helper).map_err(unavailable)?;
    let runner_hash = runtime::hash(&helper).map_err(unavailable)?;
    if runtime::available(mode).map_err(unavailable)? != runner_hash {
        return Err(unavailable(
            "the installed Windows sandbox differs from this product; an approved installation update is required",
        ));
    }
    let installed_root =
        std::fs::canonicalize(runtime::root().map_err(unavailable)?).map_err(unavailable)?;
    for grant in scope.grants() {
        let root = std::fs::canonicalize(grant.dir().canonical_path()).map_err(unavailable)?;
        if root.starts_with(&installed_root) || installed_root.starts_with(&root) {
            return Err(SandboxError::UnsupportedPolicy(
                "a command cannot grant access to its own sandbox installation".into(),
            ));
        }
    }
    Ok(PreparedCommand::sandboxed(
        command,
        Execution {
            runner_hash,
            files,
            host_acl_scope: Some(authority),
            acl_changes: policy.host_acl_changes(),
            command: command_line,
            working_directory: command
                .working_directory()
                .to_str()
                .ok_or_else(|| unavailable("working directory must be Unicode"))?
                .into(),
            env: Vec::new(),
            mode,
            proxy_port,
        },
    ))
}

impl SandboxLaunch for Execution {
    fn spawn(
        mut self: Box<Self>,
        environment: &[(String, String)],
    ) -> Result<ProcessHandle, SandboxError> {
        self.env = environment
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        spawn(&self)
            .map(ProcessHandle::new)
            .map_err(|message| SandboxError::StartFailed {
                timing: zeta_sandboxing::SandboxDenialTiming::ProcessMayHaveStarted,
                message,
            })
    }
}

struct Directory(PathBuf);
impl Directory {
    fn new(
        root: &std::path::Path,
        owner: &str,
        user: &str,
        capability: &str,
    ) -> Result<Self, String> {
        let runs = root.join("runs");
        std::fs::create_dir_all(&runs).map_err(|error| error.to_string())?;
        let path = runs.join(user);
        std::fs::create_dir(&path).map_err(|error| error.to_string())?;
        let directory = Self(path);
        let sd = win::descriptor(&format!(
            "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{owner})(A;OICI;0x1301bf;;;{user})(A;OICI;0x1301bf;;;{capability})"
        ))?;
        if unsafe {
            SetFileSecurityW(
                win::wide(&directory.0).as_ptr(),
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                sd.0,
            )
        } == 0
        {
            return Err(win::error("SetFileSecurityW(execution directory)"));
        }
        Ok(directory)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        // The path is constructed from the private runtime and an OS-random name.
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("Windows execution directory cleanup failed: {error}");
        }
    }
}

struct Resources {
    job: job::Job,
    filesystem: filesystem::Filesystem,
    _proxy: Option<proxy::Proxy>,
    _desktop: desktop::Desktop,
    _directory_pin: win::Handle,
    _directory: Directory,
    _lease: runtime::Lease,
}

struct Process {
    process: Option<win::Handle>,
    pid: u32,
    stdin: Option<File>,
    stdout: Option<File>,
    stderr: Option<File>,
    resources: Option<Resources>,
    exit: Option<i32>,
}
unsafe impl Send for Process {}

/// Start the already selected account implementation without changing backends.
fn spawn(request: &Execution) -> Result<Process, String> {
    let (mode, destination) = (request.mode, request.proxy_port);
    let lease = runtime::lease(mode)?;
    if lease.runner_hash != request.runner_hash {
        return Err("sandbox installation changed after preparation".into());
    }
    let owner = win::current_user()?;
    let mut random = [0u8; 16];
    getrandom::getrandom(&mut random).map_err(|error| error.to_string())?;
    let capability = format!(
        "S-1-5-21-{}",
        random
            .chunks_exact(4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()).to_string())
            .collect::<Vec<_>>()
            .join("-")
    );
    let directory = Directory::new(&lease.root, &owner, &lease.account.sid, &capability)?;
    let desktop = desktop::Desktop::new(&owner, &lease.account.sid, &capability)?;
    let directory_pin = filesystem::pin(&directory.0)?;
    let filesystem = filesystem::Filesystem::prepare(
        request,
        &lease.account.sid,
        &capability,
        &lease.root.join("acl").join(&lease.account.sid),
    )?;
    let proxy = destination
        .map(|port| {
            proxy::Proxy::start(
                lease.account.proxy_port,
                port,
                lease.account.sid.clone(),
                capability.clone(),
            )
        })
        .transpose()?;
    let job = job::Job::new(&lease.account.name)?;
    let pipes = process::Pipes::new(&owner, &lease.account.sid)?;
    let mut environment = BTreeMap::new();
    for entry in &request.env {
        let (name, value) = entry.split_once('=').ok_or("invalid environment entry")?;
        if name.is_empty() || name.contains('\0') || value.contains('\0') {
            return Err("invalid environment entry".into());
        }
        environment.insert(name.to_ascii_uppercase(), value.to_owned());
    }
    for name in [
        "TEMP",
        "TMP",
        "USERPROFILE",
        "HOME",
        "APPDATA",
        "LOCALAPPDATA",
    ] {
        environment.insert(name.to_owned(), directory.0.to_string_lossy().into_owned());
    }
    if mode == NetworkMode::Managed {
        let http = format!("http://127.0.0.1:{}", lease.account.proxy_port);
        for name in ["HTTP_PROXY", "HTTPS_PROXY", "WS_PROXY", "WSS_PROXY"] {
            environment.insert(name.into(), http.clone());
        }
        environment.insert(
            "ALL_PROXY".into(),
            format!("socks5h://127.0.0.1:{}", lease.account.proxy_port),
        );
        environment.insert("NO_PROXY".into(), String::new());
    }
    let worker = process::Request {
        version: 3,
        owner,
        account: lease.account.sid.clone(),
        capability,
        command: request.command.clone(),
        cwd: request.working_directory.clone(),
        environment: environment
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect(),
        pipes: pipes.names.clone(),
        reply: directory.0.join("reply.json"),
        desktop: desktop.name.clone(),
    };
    let mut result = Process {
        process: None,
        pid: 0,
        stdin: None,
        stdout: None,
        stderr: None,
        resources: Some(Resources {
            job,
            filesystem,
            _proxy: proxy,
            _desktop: desktop,
            _directory_pin: directory_pin,
            _directory: directory,
            _lease: lease,
        }),
        exit: None,
    };
    let resources = result.resources.as_ref().unwrap();
    let child = process::spawn(
        &resources._lease.account,
        &resources._lease.runner,
        &resources._directory.0,
        &worker,
        pipes,
        &resources.job,
    )?;
    result.pid = child.pid;
    result.process = Some(child.process);
    result.stdin = Some(child.stdin);
    result.stdout = Some(child.stdout);
    result.stderr = Some(child.stderr);
    Ok(result)
}

impl Process {
    fn finish(&mut self) -> io::Result<()> {
        if let Some(resources) = self.resources.as_mut() {
            resources
                .job
                .terminate_and_wait(u32::MAX)
                .map_err(|error| io::Error::other(error.to_string()))?;
            resources.filesystem.restore().map_err(io::Error::other)?;
        }
        self.resources.take();
        Ok(())
    }
}

impl SandboxProcess for Process {
    fn take_stdin(&mut self) -> Option<Box<dyn Write + Send>> {
        self.stdin.take().map(|file| Box::new(file) as _)
    }
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.stdout.take().map(|file| Box::new(file) as _)
    }
    fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>> {
        self.stderr.take().map(|file| Box::new(file) as _)
    }
    fn try_wait(&mut self) -> io::Result<Option<SandboxProcessExitStatus>> {
        if let Some(exit) = self.exit {
            return Ok(Some(SandboxProcessExitStatus::Code(exit)));
        }
        let process = self
            .process
            .as_ref()
            .ok_or_else(|| io::Error::other("child has not started"))?;
        match unsafe { WaitForSingleObject(process.0, 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0;
                if unsafe { GetExitCodeProcess(process.0, &mut code) } == 0 {
                    return Err(io::Error::last_os_error());
                }
                self.exit = Some(code as i32);
                self.finish()?;
                Ok(Some(SandboxProcessExitStatus::Code(code as i32)))
            }
            _ => Err(io::Error::last_os_error()),
        }
    }
    fn close(&mut self) -> io::Result<()> {
        self.finish()
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("Windows sandbox cleanup requires recovery: {error}");
            if let Some(resources) = self.resources.take() {
                std::mem::forget(resources);
            }
        }
    }
}

pub(super) fn run(arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<(), String> {
    let arguments = arguments
        .map(|value| {
            value
                .into_string()
                .map_err(|_| "arguments must be Unicode".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    match arguments.as_slice() {
        [command, path] if command == "run" => process::run(std::path::Path::new(path)),
        [command] if command == "status" => { runtime::available(NetworkMode::Denied)?; println!("Windows sandbox is ready."); Ok(()) },
        [command, operation, slots, count] if command == "plan" && operation == "setup" && slots == "--slots" => runtime::print_plan(runtime::setup_plan(count.parse().map_err(|_| "slots must be a number")?)?),
        [command, operation] if command == "plan" && operation == "remove" => runtime::print_plan(runtime::removal_plan()?),
        [command, slots, count, approve, digest] if command == "setup" && slots == "--slots" && approve == "--approve" => runtime::setup(count.parse().map_err(|_| "slots must be a number")?, digest),
        [command, approve, digest] if command == "remove" && approve == "--approve" => runtime::remove(digest),
        _ => Err("usage: zeta-windows-sandbox plan setup --slots N | setup --slots N --approve SHA256 | plan remove | remove --approve SHA256 | status".into()),
    }
}
