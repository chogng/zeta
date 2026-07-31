use super::fs_watcher::FileSystemWatcher;
use super::git_runtime::{GitRuntime, GitWatcher};
use super::{AppServer, AppServerThreadUpdates, RpcError};
use crate::local_tools::compose_local_tools;
use crate::tool_composition::{ToolPort, combine_tool_ports};
use crate::workspace_search::WorkspaceSearchService;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_core::TurnExecutor;
use zeta_file_system::{LocalFileSystem, WorkspaceFileSystem};
use zeta_protocol::TurnStatus;
use zeta_sandboxing::WorkspaceRoot;

pub(super) struct WorkspaceRuntime {
    pub(super) root: Option<PathBuf>,
    pub(super) file_system: Option<Arc<dyn WorkspaceFileSystem>>,
    pub(super) _file_system_watcher: Option<FileSystemWatcher>,
    pub(super) _git_watcher: Option<GitWatcher>,
    pub(super) git: Option<Arc<GitRuntime>>,
    pub(super) workspace_search: Option<Arc<WorkspaceSearchService>>,
    pub(super) terminals: Option<Arc<crate::terminal_service::TerminalService>>,
    pub(super) turn_executor: TurnExecutor,
}

impl WorkspaceRuntime {
    pub(super) fn empty(turn_executor: TurnExecutor) -> Self {
        Self {
            root: None,
            file_system: None,
            _file_system_watcher: None,
            _git_watcher: None,
            git: None,
            workspace_search: None,
            terminals: None,
            turn_executor,
        }
    }
}

pub(super) struct LocalWorkspaceHost {
    mcp: Option<ToolPort>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceRuntimeError {
    Unavailable,
    Busy,
    Failed(String),
}

impl fmt::Display for WorkspaceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("local Workspace switching is unavailable"),
            Self::Busy => formatter.write_str("a Turn is still active in the current Workspace"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for WorkspaceRuntimeError {}

impl AppServer {
    pub(crate) fn with_local_workspace_host(
        mut self,
        mcp: Option<ToolPort>,
    ) -> Result<Self, WorkspaceRuntimeError> {
        if self.local_workspace_host.is_some() {
            return Err(WorkspaceRuntimeError::Failed(
                "local Workspace host is already installed".into(),
            ));
        }
        let executor = self.compose_turn_executor(mcp.iter().cloned().collect())?;
        self.workspace_runtime
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .turn_executor = executor;
        self.local_workspace_host = Some(LocalWorkspaceHost { mcp });
        Ok(self)
    }

    pub(crate) fn switch_local_workspace_root(
        &self,
        root: PathBuf,
    ) -> Result<PathBuf, WorkspaceRuntimeError> {
        let host = self
            .local_workspace_host
            .as_ref()
            .ok_or(WorkspaceRuntimeError::Unavailable)?;
        let _workspace_authority = self.workspace_authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        self.ensure_workspace_switch_is_idle()?;
        let workspace = WorkspaceRoot::open(root)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let local = compose_local_tools(workspace.clone())
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        self.commit_workspace_runtime(workspace, local, host)
    }

    fn commit_workspace_runtime(
        &self,
        workspace: WorkspaceRoot,
        local: crate::local_tools::LocalToolComposition,
        host: &LocalWorkspaceHost,
    ) -> Result<PathBuf, WorkspaceRuntimeError> {
        let file_system: Arc<dyn WorkspaceFileSystem> =
            Arc::new(LocalFileSystem::new(workspace.clone()));
        let git = GitRuntime::new(workspace.path().to_path_buf(), Arc::clone(&self.updates))
            .map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to initialize Git runtime".into())
            })?;
        let mut ports = vec![ToolPort::local(local.tools, local.policy)];
        if let Some(mcp) = &host.mcp {
            ports.push(mcp.clone());
        }
        let turn_executor = self.compose_turn_executor(ports)?;
        let canonical_root = workspace.path().to_path_buf();

        let mut current = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if current.root.as_ref() == Some(&canonical_root) {
            return Ok(canonical_root);
        }
        let workspace_search = current.workspace_search.clone().unwrap_or_else(|| {
            Arc::new(WorkspaceSearchService::new(
                workspace.clone(),
                local.ripgrep.clone(),
            ))
        });
        let terminals = match &current.terminals {
            Some(terminals) => Arc::clone(terminals),
            None => Arc::new(
                crate::terminal_service::TerminalService::new(canonical_root.clone()).map_err(
                    |_| {
                        WorkspaceRuntimeError::Failed(
                            "failed to initialize terminal runtime".into(),
                        )
                    },
                )?,
            ),
        };
        workspace_search.switch_workspace(workspace);
        terminals.switch_workspace_root(canonical_root.clone());
        let next = WorkspaceRuntime {
            root: Some(canonical_root.clone()),
            file_system: Some(file_system),
            _file_system_watcher: None,
            _git_watcher: None,
            git: Some(Arc::clone(&git)),
            workspace_search: Some(workspace_search),
            terminals: Some(terminals),
            turn_executor,
        };
        let previous = std::mem::replace(&mut *current, next);
        drop(current);
        drop(previous);
        let file_system_watcher =
            FileSystemWatcher::start(canonical_root.clone(), Arc::clone(&self.updates));
        let git_watcher = git.start_watching();
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        runtime._file_system_watcher = Some(file_system_watcher);
        runtime._git_watcher = Some(git_watcher);
        Ok(canonical_root)
    }

    pub(super) fn workspace_features(&self) -> (bool, bool, bool, bool) {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let switchable = self.local_workspace_host.is_some();
        (
            switchable || runtime.file_system.is_some(),
            switchable || runtime.git.is_some(),
            switchable || runtime.workspace_search.is_some(),
            switchable || runtime.terminals.is_some(),
        )
    }

    pub(super) fn file_system_service(&self) -> Result<Arc<dyn WorkspaceFileSystem>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .file_system
            .clone()
            .ok_or_else(|| RpcError::new(-32040, AppServerErrorName::FileSystemUnavailable))
    }

    pub(super) fn git_runtime_service(&self) -> Result<Arc<GitRuntime>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .git
            .clone()
            .ok_or_else(|| RpcError::new(-32060, AppServerErrorName::GitUnavailable))
    }

    pub(super) fn workspace_search_service(&self) -> Result<Arc<WorkspaceSearchService>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .workspace_search
            .clone()
            .ok_or_else(|| RpcError::new(-32050, AppServerErrorName::SearchUnavailable))
    }

    pub(super) fn terminal_service(
        &self,
    ) -> Result<Arc<crate::terminal_service::TerminalService>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .terminals
            .clone()
            .ok_or_else(|| RpcError::new(-32060, AppServerErrorName::TerminalUnavailable))
    }

    pub(super) fn configured_terminal_service(
        &self,
    ) -> Option<Arc<crate::terminal_service::TerminalService>> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .terminals
            .clone()
    }

    pub(super) fn turn_executor_snapshot(&self) -> TurnExecutor {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .turn_executor
            .clone()
    }

    fn compose_turn_executor(
        &self,
        ports: Vec<ToolPort>,
    ) -> Result<TurnExecutor, WorkspaceRuntimeError> {
        let tools = combine_tool_ports(ports)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let executor = match tools {
            Some(tools) => TurnExecutor::new(
                self.sessions.threads().clone(),
                Arc::clone(&self.model),
                tools.tools,
                tools.policy,
            ),
            None => TurnExecutor::without_tools(
                self.sessions.threads().clone(),
                Arc::clone(&self.model),
            ),
        };
        Ok(
            executor.with_thread_updates(Arc::new(AppServerThreadUpdates {
                updates: Arc::clone(&self.updates),
            })),
        )
    }

    fn ensure_workspace_switch_is_idle(&self) -> Result<(), WorkspaceRuntimeError> {
        let threads = self
            .sessions
            .threads()
            .list_threads()
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let busy = threads.iter().flat_map(|thread| &thread.turns).any(|turn| {
            matches!(
                turn.status,
                TurnStatus::Created
                    | TurnStatus::Running
                    | TurnStatus::WaitingForApproval
                    | TurnStatus::WaitingForUserInput
                    | TurnStatus::WaitingForCapability
                    | TurnStatus::Cancelling
            )
        });
        if busy {
            return Err(WorkspaceRuntimeError::Busy);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "workspace_runtime_tests.rs"]
mod tests;
