use crate::{
    PreparedCommand, SandboxCommand, SandboxError, SandboxKind, SandboxPolicy,
    SandboxProcessDenial, SandboxProcessExitStatus,
};
use zeta_workspace::WorkspaceRoot;

/// Converts a validated command and policy into a platform-enforced launch command.
///
/// Implementations must fail closed when the requested restrictions cannot be enforced. They
/// receive a command whose working directory has already been canonicalized inside `workspace`.
pub trait SandboxBackend: Send + Sync {
    fn kind(&self) -> SandboxKind;

    /// Classifies a non-successful prepared process result as sandbox enforcement or an ordinary
    /// command failure.
    ///
    /// Implementations must return `Some` only for backend-specific evidence they recognize.
    /// Generic non-zero exits are not sandbox denials.
    fn classify_denial(
        &self,
        exit_status: SandboxProcessExitStatus,
        stdout: &str,
        stderr: &str,
    ) -> Option<SandboxProcessDenial> {
        let _ = (exit_status, stdout, stderr);
        None
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        workspace: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError>;
}

/// Validates command paths and delegates platform-specific sandbox construction.
pub struct SandboxManager<B> {
    workspace: WorkspaceRoot,
    backend: B,
}

impl<B: SandboxBackend> SandboxManager<B> {
    pub fn new(workspace: WorkspaceRoot, backend: B) -> Self {
        Self { workspace, backend }
    }

    pub fn backend_kind(&self) -> SandboxKind {
        self.backend.kind()
    }

    pub fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
    ) -> Result<PreparedCommand, SandboxError> {
        let working_directory = self
            .workspace
            .resolve_existing(command.working_directory())?;
        let command = command.with_working_directory(working_directory);
        self.backend.prepare(&command, policy, &self.workspace)
    }

    pub fn classify_denial(
        &self,
        exit_status: SandboxProcessExitStatus,
        stdout: &str,
        stderr: &str,
    ) -> Option<SandboxProcessDenial> {
        self.backend.classify_denial(exit_status, stdout, stderr)
    }
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
