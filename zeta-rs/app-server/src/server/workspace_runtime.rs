use super::code_index_runtime::CodeIndexRuntime;
use super::fs_watcher::FileSystemWatcher;
use super::git_runtime::{GitRuntime, GitWatcher};
use super::workspace_customizations::WorkspaceCustomizations;
use super::{AppServer, AppServerThreadUpdates, RpcError};
use crate::local_tools::compose_local_tools;
use crate::tool_composition::{ReloadableToolPorts, ToolPort, combine_tool_ports};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_code_index::CodeIndexStorage;
use zeta_code_index_cloud::CloudCodeIndexController;
use zeta_code_index_cloud::CloudCodeIndexStorage;
use zeta_config::ConfigStore;
use zeta_core::{InterruptTurnRequest, SequenceExpectation, ThreadController, TurnExecutor};
use zeta_file_system::{LocalFileSystem, WorkspaceFileSystem};
use zeta_protocol::{CommandId, TurnStatus};
use zeta_search::SearchService;
use zeta_workspace::{
    WorkspaceAuthorization, WorkspaceCapability, WorkspaceRoot, WorkspaceTrustDecision,
};

pub(super) struct WorkspaceRuntime {
    authorization: Option<WorkspaceAuthorization>,
    pub(super) file_system: Option<Arc<dyn WorkspaceFileSystem>>,
    pub(super) _file_system_watcher: Option<FileSystemWatcher>,
    pub(super) _git_watcher: Option<GitWatcher>,
    pub(super) git: Option<Arc<GitRuntime>>,
    pub(super) workspace_search: Option<Arc<SearchService>>,
    pub(super) code_index: Option<Arc<CodeIndexRuntime>>,
    pub(super) cloud_code_index: Option<Arc<CloudCodeIndexController>>,
    pub(super) _customizations: Option<Arc<WorkspaceCustomizations>>,
    pub(super) terminals: Option<Arc<crate::terminal_service::TerminalService>>,
    pub(super) turn_executor: TurnExecutor,
}

impl WorkspaceRuntime {
    pub(super) fn empty(turn_executor: TurnExecutor) -> Self {
        Self {
            authorization: None,
            file_system: None,
            _file_system_watcher: None,
            _git_watcher: None,
            git: None,
            workspace_search: None,
            code_index: None,
            cloud_code_index: None,
            _customizations: None,
            terminals: None,
            turn_executor,
        }
    }
}

pub(super) struct LocalWorkspaceHost {
    tools: Arc<WorkspaceToolPorts>,
    trust: WorkspaceSwitchTrustPolicy,
}

#[derive(Clone)]
pub(crate) struct WorkspaceRuntimeControl {
    authority_gate: Arc<Mutex<()>>,
    runtime: Arc<RwLock<WorkspaceRuntime>>,
    tools: Arc<WorkspaceToolPorts>,
    threads: Arc<ThreadController>,
    updates: Arc<super::update_broker::UpdateBroker>,
}

impl WorkspaceRuntimeControl {
    pub(crate) fn reconcile_user_trust(
        &self,
        config: &zeta_config::ResolvedConfig,
    ) -> Result<(), WorkspaceRuntimeError> {
        let _authority = self.authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let mut runtime = self
            .runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(authorization) = runtime.authorization.as_ref() else {
            return Ok(());
        };
        if !matches!(
            authorization.decision(),
            WorkspaceTrustDecision::Trusted(
                zeta_workspace::WorkspaceTrustSource::ExplicitUserDecision
            )
        ) || matches!(
            config
                .workspace_trust
                .decision_for(&authorization.root().trust_id()),
            WorkspaceTrustDecision::Trusted(_)
        ) {
            return Ok(());
        }

        let root = authorization.root().clone();
        let cloud_code_index = runtime.cloud_code_index.clone();
        authorization.revoke();
        let terminals = runtime.terminals.take();
        let search = runtime.workspace_search.take();
        let git = runtime.git.take();
        let git_watcher = runtime._git_watcher.take();
        runtime.cloud_code_index = None;
        runtime.authorization = Some(WorkspaceAuthorization::new(
            root,
            WorkspaceTrustDecision::Restricted,
        ));
        drop(runtime);

        let tool_result = self.tools.replace_local(None);
        if let Some(terminals) = terminals {
            terminals.terminate_all();
        }
        if let Some(search) = search {
            search.cancel_all();
        }
        drop(git_watcher);
        drop(git);
        let interrupt_result = self.interrupt_active_turns();
        if let Some(controller) = cloud_code_index
            && controller.revoke().is_err()
        {
            log::warn!("cloud code-index deletion remains pending after trust revocation");
        }
        tool_result?;
        interrupt_result
    }

    fn interrupt_active_turns(&self) -> Result<(), WorkspaceRuntimeError> {
        for snapshot in self
            .threads
            .list_threads()
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?
        {
            let Some(turn) = snapshot.turns.iter().find(|turn| {
                matches!(
                    turn.status,
                    TurnStatus::Created
                        | TurnStatus::Running
                        | TurnStatus::WaitingForApproval
                        | TurnStatus::WaitingForUserInput
                        | TurnStatus::WaitingForCapability
                        | TurnStatus::Cancelling
                )
            }) else {
                continue;
            };
            let after_sequence = snapshot.sequence;
            self.threads
                .interrupt_turn(
                    &snapshot.thread_id,
                    InterruptTurnRequest {
                        command_id: CommandId::new(format!(
                            "workspace-trust-revocation-{}-{}",
                            snapshot.thread_id, turn.turn_id
                        ))
                        .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?,
                        expected_sequence: SequenceExpectation::Exact(after_sequence),
                        turn_id: turn.turn_id.clone(),
                    },
                )
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
            let updates = self
                .threads
                .thread_updates_after(&snapshot.thread_id, after_sequence)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
            self.updates.publish_thread(&snapshot.thread_id, &updates);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) enum WorkspaceSwitchTrustPolicy {
    /// Keeps safe Workspace activation disabled until a host trust owner supplies a decision.
    #[cfg(test)]
    Restricted,
    /// Treats the local protocol connection as the host selection authority and binds every
    /// accepted selection to its exact canonical root.
    #[cfg(test)]
    TrustHostSelectedRoots(zeta_workspace::WorkspaceTrustSource),
    /// Resolves each client-requested root against the durable user Config authority.
    UserConfig(Arc<ConfigStore>),
}

impl WorkspaceSwitchTrustPolicy {
    fn authorize(
        &self,
        root: WorkspaceRoot,
    ) -> Result<WorkspaceAuthorization, WorkspaceRuntimeError> {
        let decision = match self {
            #[cfg(test)]
            Self::Restricted => WorkspaceTrustDecision::Restricted,
            #[cfg(test)]
            Self::TrustHostSelectedRoots(source) => WorkspaceTrustDecision::Trusted(*source),
            Self::UserConfig(config) => config
                .read_snapshot()
                .map_err(|error| WorkspaceRuntimeError::Failed(error.0))?
                .values
                .workspace_trust
                .decision_for(&root.trust_id()),
        };
        Ok(WorkspaceAuthorization::new(root, decision))
    }
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
    TrustRequired,
    Failed(String),
}

impl fmt::Display for WorkspaceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("local Workspace switching is unavailable"),
            Self::Busy => formatter.write_str("a Turn is still active in the current Workspace"),
            Self::TrustRequired => {
                formatter.write_str("the Workspace is not trusted for executable local services")
            }
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for WorkspaceRuntimeError {}

impl AppServer {
    pub(crate) fn with_local_workspace_host(
        mut self,
        mcp: Option<ToolPort>,
        trust: WorkspaceSwitchTrustPolicy,
    ) -> Result<Self, WorkspaceRuntimeError> {
        if self.local_workspace_host.is_some() {
            return Err(WorkspaceRuntimeError::Failed(
                "local Workspace host is already installed".into(),
            ));
        }
        let tools = WorkspaceToolPorts::new(mcp)?;
        let mut executor = TurnExecutor::new(
            self.sessions.threads().clone(),
            Arc::clone(&self.model),
            tools.reloadable.tools(),
            tools.reloadable.policy(),
        )
        .with_thread_updates(Arc::new(AppServerThreadUpdates {
            updates: Arc::clone(&self.updates),
        }));
        if let Some(skills) = &self.skills {
            executor = executor.with_skill_instructions_provider(skills.clone());
        }
        self.workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .turn_executor = executor;
        self.local_workspace_host = Some(LocalWorkspaceHost { tools, trust });
        Ok(self)
    }

    pub(crate) fn local_workspace_tool_ports(&self) -> Option<Arc<WorkspaceToolPorts>> {
        self.local_workspace_host
            .as_ref()
            .map(|host| Arc::clone(&host.tools))
    }

    pub(crate) fn workspace_runtime_control(&self) -> Option<WorkspaceRuntimeControl> {
        self.local_workspace_host
            .as_ref()
            .map(|host| WorkspaceRuntimeControl {
                authority_gate: Arc::clone(&self.workspace_authority_gate),
                runtime: Arc::clone(&self.workspace_runtime),
                tools: Arc::clone(&host.tools),
                threads: self.sessions.threads().clone(),
                updates: Arc::clone(&self.updates),
            })
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
        let authorization = host.trust.authorize(workspace)?;
        self.activate_local_workspace(authorization, host)
    }

    pub(crate) fn activate_host_configured_workspace_root(
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
        let authorization = WorkspaceAuthorization::new(
            workspace,
            WorkspaceTrustDecision::Trusted(
                zeta_workspace::WorkspaceTrustSource::HostConfiguration,
            ),
        );
        self.activate_local_workspace(authorization, host)
    }

    fn activate_local_workspace(
        &self,
        authorization: WorkspaceAuthorization,
        host: &LocalWorkspaceHost,
    ) -> Result<PathBuf, WorkspaceRuntimeError> {
        if self.workspace_authority_is_current(&authorization) {
            return Ok(authorization.root().canonical_path().to_path_buf());
        }
        if authorization.decision() == WorkspaceTrustDecision::Restricted {
            return self.commit_restricted_workspace_runtime(authorization, host);
        }
        let execution = authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let local = compose_local_tools(execution)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        self.commit_trusted_workspace_runtime(authorization, local, host)
    }

    fn commit_restricted_workspace_runtime(
        &self,
        authorization: WorkspaceAuthorization,
        host: &LocalWorkspaceHost,
    ) -> Result<PathBuf, WorkspaceRuntimeError> {
        let workspace = authorization.root().clone();
        let canonical_root = workspace.canonical_path().to_path_buf();
        self.revoke_cloud_index_for_restricted_root(&workspace);
        let file_system: Arc<dyn WorkspaceFileSystem> =
            Arc::new(LocalFileSystem::new(workspace.clone()));
        let code_index = self.open_code_index_runtime(workspace.clone())?;
        self.retry_persisted_cloud_index_deletion(&code_index);
        let customizations = WorkspaceCustomizations::discover(&canonical_root);
        let file_system_watcher = FileSystemWatcher::start_with_observers(
            workspace,
            Arc::clone(&self.updates),
            Arc::clone(&code_index),
            customizations.clone(),
        )
        .map_err(|error| {
            WorkspaceRuntimeError::Failed(format!(
                "failed to initialize filesystem watcher: {error}"
            ))
        })?;

        host.tools.replace_local(None)?;
        self.bind_workspace_skills(&canonical_root)?;
        let mut current = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let next = WorkspaceRuntime {
            authorization: Some(authorization),
            file_system: Some(file_system),
            _file_system_watcher: Some(file_system_watcher),
            _git_watcher: None,
            git: None,
            workspace_search: None,
            code_index: Some(code_index),
            cloud_code_index: None,
            _customizations: Some(Arc::clone(&customizations)),
            terminals: None,
            turn_executor: current
                .turn_executor
                .clone()
                .with_instructions_provider(customizations),
        };
        let previous = std::mem::replace(&mut *current, next);
        drop(current);
        retire_workspace_runtime(previous, None, None);
        Ok(canonical_root)
    }

    fn commit_trusted_workspace_runtime(
        &self,
        authorization: WorkspaceAuthorization,
        local: crate::local_tools::LocalToolComposition,
        host: &LocalWorkspaceHost,
    ) -> Result<PathBuf, WorkspaceRuntimeError> {
        let workspace = authorization.root().clone();
        let file_system: Arc<dyn WorkspaceFileSystem> =
            Arc::new(LocalFileSystem::new(workspace.clone()));
        let code_index = self.open_code_index_runtime(workspace.clone())?;
        let cloud_code_index = self.open_cloud_code_index_controller(&code_index);
        let canonical_root = workspace.canonical_path().to_path_buf();
        let customizations = WorkspaceCustomizations::discover(&canonical_root);
        let repository_mutation = authorization
            .require(WorkspaceCapability::MutateRepository)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let git =
            GitRuntime::new(repository_mutation, Arc::clone(&self.updates)).map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to initialize Git runtime".into())
            })?;
        let file_system_watcher = FileSystemWatcher::start_with_observers(
            workspace.clone(),
            Arc::clone(&self.updates),
            Arc::clone(&code_index),
            customizations.clone(),
        )
        .map_err(|error| {
            WorkspaceRuntimeError::Failed(format!(
                "failed to initialize filesystem watcher: {error}"
            ))
        })?;
        let local_port = ToolPort::local(local.tools, local.policy);
        let (existing_search, existing_terminals) = {
            let current = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (current.workspace_search.clone(), current.terminals.clone())
        };
        let workspace_search = existing_search.unwrap_or_else(|| {
            Arc::new(SearchService::new(workspace.clone(), local.ripgrep.clone()))
        });
        workspace_search.cancel_all();
        workspace_search.switch_workspace(workspace);
        let terminal_capability = authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let terminals = match existing_terminals {
            Some(terminals) => {
                terminals.terminate_all();
                terminals
                    .switch_workspace(terminal_capability)
                    .map_err(|_| {
                        WorkspaceRuntimeError::Failed("failed to switch terminal runtime".into())
                    })?;
                terminals
            }
            None => Arc::new(
                crate::terminal_service::TerminalService::new(terminal_capability).map_err(
                    |_| {
                        WorkspaceRuntimeError::Failed(
                            "failed to initialize terminal runtime".into(),
                        )
                    },
                )?,
            ),
        };
        host.tools.replace_local(Some(local_port))?;
        self.bind_workspace_skills(&canonical_root)?;
        let mut current = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let next = WorkspaceRuntime {
            authorization: Some(authorization),
            file_system: Some(file_system),
            _file_system_watcher: Some(file_system_watcher),
            _git_watcher: None,
            git: Some(Arc::clone(&git)),
            workspace_search: Some(Arc::clone(&workspace_search)),
            code_index: Some(code_index),
            cloud_code_index,
            _customizations: Some(Arc::clone(&customizations)),
            terminals: Some(Arc::clone(&terminals)),
            turn_executor: current
                .turn_executor
                .clone()
                .with_instructions_provider(customizations),
        };
        let previous = std::mem::replace(&mut *current, next);
        drop(current);
        retire_workspace_runtime(previous, Some(&workspace_search), Some(&terminals));
        let git_watcher = git.start_watching();
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        runtime._git_watcher = Some(git_watcher);
        Ok(canonical_root)
    }

    fn bind_workspace_skills(
        &self,
        workspace_root: &std::path::Path,
    ) -> Result<(), WorkspaceRuntimeError> {
        let Some(skills) = &self.skills else {
            return Ok(());
        };
        skills
            .bind_workspace_root(workspace_root.to_path_buf())
            .map(|_| ())
            .map_err(|error| {
                WorkspaceRuntimeError::Failed(format!(
                    "failed to bind Workspace Skill source: {error}"
                ))
            })
    }

    fn workspace_authority_is_current(&self, authorization: &WorkspaceAuthorization) -> bool {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()
            .is_some_and(|current| {
                current.root() == authorization.root()
                    && current.decision() == authorization.decision()
                    && current.is_active() == authorization.is_active()
            })
    }

    pub(super) fn workspace_features(&self) -> (bool, bool, bool, bool, bool, bool) {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let switchable = self.local_workspace_host.is_some();
        (
            switchable || runtime.file_system.is_some(),
            switchable || runtime.git.is_some(),
            switchable || runtime.workspace_search.is_some(),
            switchable || runtime.code_index.is_some(),
            (switchable && !self.cloud_code_index_providers.is_empty())
                || runtime.cloud_code_index.is_some(),
            switchable || runtime.terminals.is_some(),
        )
    }

    fn open_code_index_runtime(
        &self,
        workspace: WorkspaceRoot,
    ) -> Result<Arc<CodeIndexRuntime>, WorkspaceRuntimeError> {
        let trust_id = workspace.trust_id();
        let digest = trust_id
            .as_str()
            .strip_prefix("sha256:")
            .unwrap_or(trust_id.as_str());
        let storage = self.code_index_storage_root.as_ref().map_or(
            CodeIndexStorage::Memory,
            |storage_root| {
                CodeIndexStorage::Persistent(storage_root.join(format!("{digest}.sqlite3")))
            },
        );
        match CodeIndexRuntime::open(workspace.clone(), storage) {
            Ok(runtime) => Ok(runtime),
            Err(persistent_error) if self.code_index_storage_root.is_some() => {
                log::warn!(
                    "persistent code-index cache is unavailable; using memory projection: {persistent_error}"
                );
                CodeIndexRuntime::open(workspace, CodeIndexStorage::Memory).map_err(|error| {
                    WorkspaceRuntimeError::Failed(format!("failed to open code index: {error}"))
                })
            }
            Err(error) => Err(WorkspaceRuntimeError::Failed(format!(
                "failed to open code index: {error}"
            ))),
        }
    }

    pub(super) fn code_index_service(&self) -> Result<Arc<CodeIndexRuntime>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .code_index
            .clone()
            .ok_or_else(|| RpcError::new(-32090, AppServerErrorName::CodeIndexUnavailable))
    }

    fn open_cloud_code_index_controller(
        &self,
        code_index: &Arc<CodeIndexRuntime>,
    ) -> Option<Arc<CloudCodeIndexController>> {
        if self.cloud_code_index_providers.is_empty() {
            return None;
        }
        let trust_id = code_index.root().trust_id();
        let digest = trust_id
            .as_str()
            .strip_prefix("sha256:")
            .unwrap_or(trust_id.as_str());
        let storage = self
            .cloud_code_index_storage_root
            .as_ref()
            .map_or(CloudCodeIndexStorage::Memory, |root| {
                CloudCodeIndexStorage::Persistent(root.join(format!("{digest}.sqlite3")))
            });
        match CloudCodeIndexController::open(
            code_index.index(),
            self.cloud_code_index_providers.clone(),
            storage,
        ) {
            Ok(controller) => Some(controller),
            Err(error) => {
                log::warn!("cloud code-index authority is unavailable: {error}");
                None
            }
        }
    }

    fn revoke_cloud_index_for_restricted_root(&self, workspace: &WorkspaceRoot) {
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let same_root = runtime
            .authorization
            .as_ref()
            .is_some_and(|authorization| authorization.root() == workspace);
        let controller = if same_root {
            if let Some(authorization) = runtime.authorization.as_ref() {
                authorization.revoke();
            }
            runtime.cloud_code_index.take()
        } else {
            None
        };
        drop(runtime);
        if let Some(controller) = controller
            && controller.revoke().is_err()
        {
            log::warn!("cloud code-index deletion remains pending after trust revocation");
        }
    }

    fn retry_persisted_cloud_index_deletion(&self, code_index: &Arc<CodeIndexRuntime>) {
        let Some(controller) = self.open_cloud_code_index_controller(code_index) else {
            return;
        };
        if controller.revoke().is_err() {
            log::warn!("cloud code-index deletion remains pending while Workspace is restricted");
        }
    }

    pub(super) fn cloud_code_index_service(
        &self,
    ) -> Result<Arc<CloudCodeIndexController>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .cloud_code_index
            .clone()
            .ok_or_else(|| RpcError::new(-32093, AppServerErrorName::CloudCodeIndexUnavailable))
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

    pub(super) fn workspace_search_service(&self) -> Result<Arc<SearchService>, RpcError> {
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

fn retire_workspace_runtime(
    mut runtime: WorkspaceRuntime,
    retained_search: Option<&Arc<SearchService>>,
    retained_terminals: Option<&Arc<crate::terminal_service::TerminalService>>,
) {
    if let Some(authorization) = runtime.authorization.take() {
        authorization.revoke();
    }
    if let Some(terminals) = runtime.terminals.take() {
        if !retained_terminals.is_some_and(|retained| Arc::ptr_eq(retained, &terminals)) {
            terminals.terminate_all();
        }
    }
    if let Some(search) = runtime.workspace_search.take() {
        if !retained_search.is_some_and(|retained| Arc::ptr_eq(retained, &search)) {
            search.cancel_all();
        }
    }
}

#[cfg(test)]
#[path = "workspace_runtime_tests.rs"]
mod tests;
