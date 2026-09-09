//! Windows AppContainer sandbox backend and packaged helper entry points.

mod discovery;
mod protocol;
mod service_protocol;

#[cfg(target_os = "windows")]
mod appcontainer;
#[cfg(target_os = "windows")]
mod runner;
#[cfg(target_os = "windows")]
mod service_client;
#[cfg(target_os = "windows")]
mod setup;

#[cfg(any(target_os = "windows", test))]
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zeta_file_access::Dir;
use zeta_install_context::InstallContext;
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxBackend, SandboxCommand, SandboxError,
    SandboxKind, SandboxPolicy, SandboxProcessDenial, SandboxProcessExitStatus,
};

pub use discovery::WindowsSandboxDiscoveryError;
pub use service_protocol::SANDBOX_SERVICE_NAME;
pub use service_protocol::SANDBOX_SERVICE_PIPE_NAME;
pub use service_protocol::SANDBOX_SERVICE_PROTOCOL_VERSION;
pub use service_protocol::SANDBOX_WORKER_EXECUTABLE_NAME;
pub use service_protocol::WindowsSandboxProvisioningAccess;
pub use service_protocol::WindowsSandboxProvisioningFrame;
pub use service_protocol::WindowsSandboxProvisioningMessage;
pub use service_protocol::WindowsSandboxProvisioningRequest;
pub use service_protocol::WindowsSandboxProvisioningResponse;
pub use service_protocol::read_provisioning_frame;
pub use service_protocol::write_provisioning_frame;

/// Materialized authority passed to the Windows AppContainer helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsSandboxPlan {
    dir: PathBuf,
    file_system: FileSystemAccess,
    network: NetworkAccess,
}

impl WindowsSandboxPlan {
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn file_system(&self) -> FileSystemAccess {
        self.file_system
    }

    pub fn network(&self) -> NetworkAccess {
        self.network
    }
}

/// Resolves shared policy into the packaged Windows AppContainer command runner.
pub struct WindowsSandbox {
    command_runner: PathBuf,
}

impl WindowsSandbox {
    /// Uses an explicit runner path. Production composition should prefer [`Self::discover`].
    pub fn new(command_runner: impl Into<PathBuf>) -> Self {
        Self {
            command_runner: command_runner.into(),
        }
    }

    /// Resolves, probes, canonicalizes, and freezes the packaged Windows runner.
    pub fn discover(context: &InstallContext) -> Result<Self, WindowsSandboxDiscoveryError> {
        discovery::discover(context)
    }

    pub fn command_runner(&self) -> &Path {
        &self.command_runner
    }

    pub fn plan(&self, policy: SandboxPolicy, dir: &Dir) -> WindowsSandboxPlan {
        WindowsSandboxPlan {
            dir: dir.canonical_path().to_path_buf(),
            file_system: policy.file_system(),
            network: policy.network(),
        }
    }
}

impl SandboxBackend for WindowsSandbox {
    fn kind(&self) -> SandboxKind {
        SandboxKind::WindowsAppContainer
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        dir: &Dir,
    ) -> Result<PreparedCommand, SandboxError> {
        if !policy.requires_platform_sandbox() {
            return Ok(PreparedCommand::unrestricted(command));
        }
        if policy.network() != NetworkAccess::Denied
            || !matches!(
                policy.file_system(),
                FileSystemAccess::ReadOnly | FileSystemAccess::DirectoryWrite
            )
        {
            return Err(SandboxError::BackendUnavailable {
                backend: SandboxKind::WindowsAppContainer,
                message: "Windows AppContainer v1 supports only read-only or dir-write filesystem access with denied network".to_owned(),
            });
        }

        let access = match policy.file_system() {
            FileSystemAccess::ReadOnly => protocol::READ_ONLY_ACCESS,
            FileSystemAccess::DirectoryWrite => protocol::DIR_WRITE_ACCESS,
            FileSystemAccess::FullAccess => unreachable!("full access was rejected above"),
        };
        let mut arguments = vec![
            protocol::ACCESS_FLAG.into(),
            access.into(),
            protocol::DIR_FLAG.into(),
            dir.canonical_path().as_os_str().to_owned(),
            protocol::CWD_FLAG.into(),
            command.working_directory().as_os_str().to_owned(),
            protocol::COMMAND_SEPARATOR.into(),
            command.program().to_owned(),
        ];
        arguments.extend(command.arguments().iter().cloned());
        Ok(PreparedCommand::new(
            SandboxKind::WindowsAppContainer,
            &self.command_runner,
            arguments,
            command.working_directory(),
        ))
    }

    fn classify_denial(
        &self,
        exit_status: SandboxProcessExitStatus,
        _stdout: &str,
        _stderr: &str,
    ) -> Option<SandboxProcessDenial> {
        (exit_status == SandboxProcessExitStatus::Code(protocol::ENFORCEMENT_FAILURE_EXIT_CODE))
            .then(|| {
                SandboxProcessDenial::before_process_start(
                    "Windows AppContainer could not establish the sandbox",
                )
            })
    }
}

#[cfg(any(target_os = "windows", test))]
fn profile_name(dir: &Path, access: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(profile_path_bytes(dir));
    digest.update([0]);
    digest.update(access.as_bytes());
    let digest = digest.finalize();
    let suffix = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mode = if access == protocol::READ_ONLY_ACCESS {
        "ro"
    } else {
        "rw"
    };
    format!("Zeta.Agent.v1.{mode}.{suffix}")
}

#[cfg(target_os = "windows")]
fn profile_path_bytes(dir: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    dir.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(all(test, not(target_os = "windows")))]
fn profile_path_bytes(dir: &Path) -> Vec<u8> {
    dir.as_os_str().as_encoded_bytes().to_vec()
}

/// Runs the packaged command-runner binary.
#[doc(hidden)]
pub fn command_runner_main() -> ! {
    #[cfg(target_os = "windows")]
    {
        runner::main()
    }
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("{} command runner is Windows-only", protocol::ERROR_PREFIX);
        std::process::exit(protocol::ENFORCEMENT_FAILURE_EXIT_CODE)
    }
}

/// Applies one request after the Windows service has authenticated and pinned its caller paths.
#[cfg(target_os = "windows")]
pub fn provision_windows_appcontainer(
    request: &WindowsSandboxProvisioningRequest,
) -> Result<(), String> {
    setup::provision(request)
}

/// Runs the service-owned worker process that applies one already-authenticated request.
#[doc(hidden)]
pub fn sandbox_worker_main() -> ! {
    #[cfg(target_os = "windows")]
    {
        let arguments = std::env::args().skip(1).collect::<Vec<_>>();
        let result = match arguments.as_slice() {
            [request] => serde_json::from_str::<WindowsSandboxProvisioningRequest>(request)
                .map_err(|error| format!("invalid provisioning worker request: {error}"))
                .and_then(|request| setup::provision(&request)),
            _ => Err("provisioning worker expects exactly one request".to_owned()),
        };
        match result {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("zeta-windows-sandbox-worker: {error}");
                std::process::exit(1)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("zeta-windows-sandbox-worker is Windows-only");
        std::process::exit(1)
    }
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;
