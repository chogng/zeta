use super::fs_watcher::FileSystemWatcher;
use super::git_runtime::{GitRuntime, GitWatcher};
use super::{AppServer, AppServerThreadUpdates, RpcError};
use crate::local_tools::compose_local_tools;
use crate::tool_composition::{ReloadableToolPorts, ToolPort, combine_tool_ports};
use crate::workspace_search::WorkspaceSearchService;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
    tools: Arc<WorkspaceToolPorts>,
}

struct WorkspaceToolPortState {
    local: Option<ToolPort>,
    mcp: Option<ToolPort>,
}

pub(crate) struct WorkspaceToolPorts {
    state: Mutex<WorkspaceToolPortState>,
    reloadable: Arc<ReloadableToolPorts>,
}

impl WorkspaceToolPorts {
    fn new(mcp: Option<ToolPort>) -> Result<Arc<Self>, WorkspaceRuntimeError> {
        let combined = combine_tool_ports(mcp.iter().cloned().collect())
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        Ok(Arc::new(Self {
            state: Mutex::new(WorkspaceToolPortState { local: None, mcp }),
            reloadable: ReloadableToolPorts::new(combined),
        }))
    }

    fn replace_local(&self, local: Option<ToolPort>) -> Result<(), WorkspaceRuntimeError> {
        self.replace(|state| state.local = local)
    }

    pub(crate) fn replace_mcp(&self, mcp: Option<ToolPort>) -> Result<(), WorkspaceRuntimeError> {
        self.replace(|state| state.mcp = mcp)
    }

    fn replace(
        &self,
        update: impl FnOnce(&mut WorkspaceToolPortState),
    ) -> Result<(), WorkspaceRuntimeError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| WorkspaceRuntimeError::Failed("Workspace tool state poisoned".into()))?;
        let mut next = WorkspaceToolPortState {
            local: state.local.clone(),
            mcp: state.mcp.clone(),
        };
        update(&mut next);
        let ports = next.local.iter().chain(next.mcp.iter()).cloned().collect();
        let combined = combine_tool_ports(ports)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        *state = next;
        self.reloadable.replace(combined);
        Ok(())
    }

    pub(crate) fn record_reconcile_failure(&self, error: impl Into<String>) {
        self.reloadable.record_reconcile_failure(error);
    }
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
        let tools = WorkspaceToolPorts::new(mcp)?;
        let executor = TurnExecutor::new(
            self.sessions.threads().clone(),
            Arc::clone(&self.model),
            tools.reloadable.tools(),
            tools.reloadable.policy(),
        )
        .with_thread_updates(Arc::new(AppServerThreadUpdates {
            updates: Arc::clone(&self.updates),
        }));
        self.workspace_runtime
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .turn_executor = executor;
        self.local_workspace_host = Some(LocalWorkspaceHost { tools });
        Ok(self)
    }

    pub(crate) fn local_workspace_tool_ports(&self) -> Option<Arc<WorkspaceToolPorts>> {
        self.local_workspace_host
            .as_ref()
            .map(|host| Arc::clone(&host.tools))
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
        let local_port = ToolPort::local(local.tools, local.policy);
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
        host.tools.replace_local(Some(local_port))?;
        let next = WorkspaceRuntime {
            root: Some(canonical_root.clone()),
            file_system: Some(file_system),
            _file_system_watcher: None,
            _git_watcher: None,
            git: Some(Arc::clone(&git)),
            workspace_search: Some(workspace_search),
            terminals: Some(terminals),
            turn_executor: current.turn_executor.clone(),
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
