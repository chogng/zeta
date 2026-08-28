//! Linux sandbox backend built on typed Bubblewrap command construction.

mod bwrap;
mod discovery;

use bwrap::BwrapCommandBuilder;
use bwrap::MountAccess;
use std::path::{Path, PathBuf};
use zeta_install_context::InstallContext;
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxBackend, SandboxCommand, SandboxError,
    SandboxKind, SandboxPolicy, SandboxProcessDenial, SandboxProcessExitStatus,
    PROTECTED_WORKSPACE_METADATA_NAMES,
};
use zeta_workspace::WorkspaceRoot;

pub use discovery::LinuxSandboxDiscoveryError;

/// Translates shared sandbox policy into a Bubblewrap launch command.
pub struct LinuxSandbox {
    bwrap_binary: PathBuf,
}

impl LinuxSandbox {
    pub fn new(bwrap_binary: impl Into<PathBuf>) -> Self {
        Self {
            bwrap_binary: bwrap_binary.into(),
        }
    }

    /// Resolves and probes the bundled or host Bubblewrap executable.
    ///
    /// Package resources precede host `PATH`; an explicit `ZETA_BWRAP_PATH` override is
    /// authoritative and therefore cannot silently fall back.
    pub fn discover(install_context: &InstallContext) -> Result<Self, LinuxSandboxDiscoveryError> {
        discovery::discover(install_context)
    }

    pub fn binary(&self) -> &Path {
        &self.bwrap_binary
    }

    /// Builds the Linux Bubblewrap launch command without spawning it.
    ///
    /// This is public so packaging and capability-probe code can inspect the exact executable and
    /// argv that will be used after selecting a Bubblewrap binary.
    pub fn prepare_command(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        workspace: &WorkspaceRoot,
    ) -> PreparedCommand {
        if !policy.requires_platform_sandbox() {
            return PreparedCommand::unrestricted(command);
        }

        let root_access = match policy.file_system() {
            FileSystemAccess::ReadOnly | FileSystemAccess::WorkspaceWrite => MountAccess::ReadOnly,
            FileSystemAccess::FullAccess => MountAccess::ReadWrite,
        };
        let mut builder = BwrapCommandBuilder::new(
            &self.bwrap_binary,
            command.program().to_owned(),
        )
        .mount(Path::new("/"), Path::new("/"), root_access);
        if policy.file_system() == FileSystemAccess::WorkspaceWrite {
            builder = builder.mount(
                workspace.canonical_path(),
                workspace.canonical_path(),
                MountAccess::ReadWrite,
            );
            for name in PROTECTED_WORKSPACE_METADATA_NAMES {
                let path = workspace.canonical_path().join(name);
                if path.exists() {
                    builder = builder.mount(&path, &path, MountAccess::ReadOnly);
                }
            }
        }
        if policy.network() == NetworkAccess::Denied {
            builder = builder.isolate_network();
        }
        let bwrap = builder
            .mount_proc()
            .mount_dev()
            .working_directory(command.working_directory())
            .inner_arguments(command.arguments().iter().cloned())
            .build();
        PreparedCommand::new(
            SandboxKind::LinuxBubblewrap,
            bwrap.program(),
            bwrap.arguments().iter().cloned(),
            command.working_directory(),
        )
    }
}

impl SandboxBackend for LinuxSandbox {
    fn kind(&self) -> SandboxKind {
        SandboxKind::LinuxBubblewrap
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        workspace: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        #[cfg(target_os = "linux")]
        {
            Ok(self.prepare_command(command, policy, workspace))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (command, policy, workspace);
            Err(SandboxError::BackendUnavailable {
                backend: SandboxKind::LinuxBubblewrap,
                message: "the Bubblewrap backend can only run on Linux".to_owned(),
            })
        }
    }

    fn classify_denial(
        &self,
        exit_status: SandboxProcessExitStatus,
        stdout: &str,
        stderr: &str,
    ) -> Option<SandboxProcessDenial> {
        if exit_status == SandboxProcessExitStatus::Code(0) {
            return None;
        }
        let output = format!("{stdout}\n{stderr}").to_ascii_lowercase();
        if output.contains("bwrap:") {
            return Some(SandboxProcessDenial::before_process_start(
                "Linux Bubblewrap could not establish the sandbox",
            ));
        }
        [
            "operation not permitted",
            "permission denied",
            "read-only file system",
            "network is unreachable",
        ]
        .iter()
        .any(|marker| output.contains(marker))
        .then(|| {
            SandboxProcessDenial::process_may_have_started(
                "Linux Bubblewrap denied the sandboxed process operation",
            )
        })
    }
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;
