//! Linux sandbox backend built on typed Bubblewrap command construction.

use std::path::{Path, PathBuf};
use zeta_bwrap::{BwrapCommandBuilder, MountAccess};
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxBackend, SandboxCommand, SandboxError,
    SandboxKind, SandboxPolicy, WorkspaceRoot,
};

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

    pub fn from_path() -> Self {
        Self::new("bwrap")
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
            builder = builder.mount(workspace.path(), workspace.path(), MountAccess::ReadWrite);
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
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;
