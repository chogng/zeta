use crate::CodexThreadAccess;
use std::path::PathBuf;
use std::sync::Arc;
use zeta_core::CoreError;

/// Current authorized workspace used to start or resume one Codex subscription Turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexTurnWorkspace {
    pub path: PathBuf,
    pub execution_scope: String,
}

/// Supplies the current authorized workspace at each Turn boundary.
///
/// Implementations must return a canonical absolute path and a stable opaque authority scope.
/// The scope is persisted with the remote thread binding and must change when workspace authority
/// changes, preventing a Codex conversation from being resumed in another workspace.
pub trait CodexTurnWorkspaceSource: Send + Sync {
    fn current_workspace(&self) -> Result<CodexTurnWorkspace, CoreError>;
}

struct FixedCodexTurnWorkspaceSource {
    workspace: CodexTurnWorkspace,
}

impl CodexTurnWorkspaceSource for FixedCodexTurnWorkspaceSource {
    fn current_workspace(&self) -> Result<CodexTurnWorkspace, CoreError> {
        Ok(self.workspace.clone())
    }
}

/// Workspace and sandbox policy used for Codex subscription Turns.
pub struct CodexTurnExecutionBackendOptions {
    pub(super) workspace: Arc<dyn CodexTurnWorkspaceSource>,
    pub(super) access: CodexThreadAccess,
}

impl CodexTurnExecutionBackendOptions {
    pub fn read_only(workspace: impl Into<PathBuf>) -> Result<Self, CoreError> {
        Self::with_access(workspace.into(), CodexThreadAccess::ReadOnly)
    }

    pub fn workspace_write(workspace: impl Into<PathBuf>) -> Result<Self, CoreError> {
        Self::with_access(workspace.into(), CodexThreadAccess::WorkspaceWrite)
    }

    fn with_access(workspace: PathBuf, access: CodexThreadAccess) -> Result<Self, CoreError> {
        if !workspace.is_absolute() {
            return Err(CoreError::InvalidInput(
                "Codex backend workspace must be absolute".into(),
            ));
        }
        let execution_scope = workspace.to_string_lossy().into_owned();
        Ok(Self {
            workspace: Arc::new(FixedCodexTurnWorkspaceSource {
                workspace: CodexTurnWorkspace {
                    path: workspace,
                    execution_scope,
                },
            }),
            access,
        })
    }

    pub fn from_source(
        workspace: Arc<dyn CodexTurnWorkspaceSource>,
        access: CodexThreadAccess,
    ) -> Self {
        Self { workspace, access }
    }
}
