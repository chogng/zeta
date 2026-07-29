//! Windows AppContainer sandbox backend and packaged helper entry points.

mod discovery;
mod protocol;

#[cfg(target_os = "windows")]
mod appcontainer;
#[cfg(target_os = "windows")]
mod runner;
#[cfg(target_os = "windows")]
mod setup;

#[cfg(any(target_os = "windows", test))]
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zeta_install_context::InstallContext;
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxBackend, SandboxCommand, SandboxError,
    SandboxKind, SandboxPolicy, SandboxProcessDenial, SandboxProcessExitStatus, WorkspaceRoot,
};

pub use discovery::WindowsSandboxDiscoveryError;

/// Materialized authority passed to the Windows AppContainer helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsSandboxPlan {
    workspace: PathBuf,
    file_system: FileSystemAccess,
    network: NetworkAccess,
}

impl WindowsSandboxPlan {
    pub fn workspace(&self) -> &Path {
        &self.workspace
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
    sandbox_setup: PathBuf,
}

impl WindowsSandbox {
    /// Uses explicit helper paths. Production composition should prefer [`Self::discover`].
    pub fn new(command_runner: impl Into<PathBuf>, sandbox_setup: impl Into<PathBuf>) -> Self {
        Self {
            command_runner: command_runner.into(),
            sandbox_setup: sandbox_setup.into(),
        }
    }

    /// Resolves, probes, canonicalizes, and freezes both packaged Windows helpers.
    pub fn discover(context: &InstallContext) -> Result<Self, WindowsSandboxDiscoveryError> {
        discovery::discover(context)
    }

    pub fn command_runner(&self) -> &Path {
        &self.command_runner
    }

    pub fn sandbox_setup(&self) -> &Path {
        &self.sandbox_setup
    }

    pub fn plan(&self, policy: SandboxPolicy, workspace: &WorkspaceRoot) -> WindowsSandboxPlan {
        WindowsSandboxPlan {
            workspace: workspace.path().to_path_buf(),
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
        workspace: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        if !policy.requires_platform_sandbox() {
            return Ok(PreparedCommand::unrestricted(command));
        }
        if policy.network() != NetworkAccess::Denied
            || !matches!(
                policy.file_system(),
                FileSystemAccess::ReadOnly | FileSystemAccess::WorkspaceWrite
            )
        {
            return Err(SandboxError::BackendUnavailable {
                backend: SandboxKind::WindowsAppContainer,
                message: "Windows AppContainer v1 supports only read-only or workspace-write filesystem access with denied network".to_owned(),
            });
        }

        let access = match policy.file_system() {
            FileSystemAccess::ReadOnly => protocol::READ_ONLY_ACCESS,
            FileSystemAccess::WorkspaceWrite => protocol::WORKSPACE_WRITE_ACCESS,
            FileSystemAccess::FullAccess => unreachable!("full access was rejected above"),
        };
        let mut arguments = vec![
            protocol::SETUP_HELPER_FLAG.into(),
            self.sandbox_setup.clone().into_os_string(),
            protocol::ACCESS_FLAG.into(),
            access.into(),
            protocol::WORKSPACE_FLAG.into(),
            workspace.path().as_os_str().to_owned(),
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
fn profile_name(workspace: &Path, access: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(profile_path_bytes(workspace));
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
fn profile_path_bytes(workspace: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    workspace
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(all(test, not(target_os = "windows")))]
fn profile_path_bytes(workspace: &Path) -> Vec<u8> {
    workspace.as_os_str().as_encoded_bytes().to_vec()
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

/// Runs the packaged sandbox-setup binary.
#[doc(hidden)]
pub fn sandbox_setup_main() -> ! {
    #[cfg(target_os = "windows")]
    {
        setup::main()
    }
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("{} sandbox setup is Windows-only", protocol::ERROR_PREFIX);
        std::process::exit(1)
    }
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;
