//! Windows restricted-token sandbox boundary.
//!
//! The shared policy is resolved here, while native token, ACL, Job Object, and network
//! enforcement will be owned by the Windows-only launcher. Until that launcher is present,
//! restricted requests fail closed.

use std::path::PathBuf;
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxBackend, SandboxCommand, SandboxError,
    SandboxKind, SandboxPolicy, WorkspaceRoot,
};

/// Materialized authority that the native Windows launcher must enforce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsSandboxPlan {
    workspace: PathBuf,
    file_system: FileSystemAccess,
    network: NetworkAccess,
}

impl WindowsSandboxPlan {
    pub fn workspace(&self) -> &std::path::Path {
        &self.workspace
    }

    pub fn file_system(&self) -> FileSystemAccess {
        self.file_system
    }

    pub fn network(&self) -> NetworkAccess {
        self.network
    }
}

/// Resolves shared policy for the Windows restricted-token launcher.
#[derive(Default)]
pub struct WindowsSandbox;

impl WindowsSandbox {
    pub fn new() -> Self {
        Self
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
        SandboxKind::WindowsRestrictedToken
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
        let _plan = self.plan(policy, workspace);
        let message = if cfg!(target_os = "windows") {
            "the restricted-token launcher has not been connected yet"
        } else {
            "the restricted-token backend can only run on Windows"
        };
        Err(SandboxError::BackendUnavailable {
            backend: SandboxKind::WindowsRestrictedToken,
            message: message.to_owned(),
        })
    }
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;
