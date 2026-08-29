use super::AppServer;
use super::AppServerThreadUpdates;
use super::CodeIndexSemanticModels;
use super::RpcError;
use super::code_index_runtime::CodeIndexRuntime;
use super::fs_watcher::FileSystemWatcher;
use super::git_runtime::GitRuntime;
use super::git_runtime::GitWatcher;
use super::goal_tool::GoalToolService;
use super::multi_agent_tools::MultiAgentToolService;
use super::semantic_index_job::AppServerSemanticIndexMetrics;
use super::semantic_index_job::SemanticIndexJobController;
use super::symbol_index_runtime::SymbolIndexRuntime;
use super::update_plan_tool::UpdatePlanToolService;
use super::workspace_customizations::WorkspaceCustomizations;
use crate::code_retrieval_context::CodeRetrievalContextSource;
use crate::code_retrieval_tool::CodeRetrievalTool;
use crate::dynamic_tools::DynamicToolCompositionError;
use crate::dynamic_tools::compose_dynamic_tools;
use crate::local_tools::AgentGrepService;
use crate::local_tools::LocalToolConfig;
use crate::local_tools::append_local_tool;
use crate::local_tools::compose_local_tools_with_config;
use crate::review::ApprovalModeActionPolicyService;
use crate::session_workspace_access::SessionWorkspaceAccess;
use crate::tool_composition::ReloadableToolPorts;
use crate::tool_composition::ToolPort;
use crate::tool_composition::ToolSearchOptions;
use crate::tool_composition::combine_tool_ports_at_generation_with_search;
use crate::tool_search_models::ToolSearchEmbeddingStatus;
use crate::tool_search_models::resolve_tool_search;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_code_index::CodeIndexStorage;
use zeta_code_index_cloud::CloudCodeIndexController;
use zeta_code_index_cloud::CloudCodeIndexStorage;
use zeta_code_index_semantic::CodeIndexEmbeddingModelId;
use zeta_code_index_semantic::CodeIndexSemanticService;
use zeta_code_index_semantic::CodeIndexSemanticStorage;
use zeta_code_index_semantic::CodeIndexVectorStore;
use zeta_code_index_semantic::SqliteCodeIndexVectorStore;
use zeta_config::ConfigStore;
use zeta_config::SemanticCodeIndexModelSelection;
use zeta_config::ToolSearchConfig;
use zeta_config::WorkspaceConfigScope;
use zeta_config::WorkspaceConfigStore;
use zeta_config::WorkspaceId;
use zeta_core::InterruptTurnRequest;
use zeta_core::MultiAgentCoordinator;
use zeta_core::SequenceExpectation;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_core::TurnExecutor;
use zeta_file_system::LocalFileSystem;
use zeta_file_system::WorkspaceFileSystem;
use zeta_hooks::DeclarativeHookRuntime;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingRuntimeRequest;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider::RerankInvoker;
use zeta_model_provider::RerankRequest;
use zeta_model_provider::RerankResponse;
use zeta_model_provider::RerankRuntimeRequest;
use zeta_model_provider::SemanticModelProvider;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::CommandId;
use zeta_protocol::ProviderId;
use zeta_protocol::SessionId;
use zeta_protocol::TurnStatus;
use zeta_shell_command::RipgrepExecutable;
use zeta_symbol_index::SymbolIndexStorage;
use zeta_tools::ToolRegistryGeneration;
use zeta_workspace::WorkspaceAuthorization;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace_access::WorkspaceAccessMutation;
use zeta_workspace_index_storage::WorkspaceIndexKind;
use zeta_workspace_index_storage::WorkspaceIndexStorage;
use zeta_workspace_search::WorkspaceSearchService;

pub(super) struct WorkspaceRuntime {
    pub(super) authorization: Option<WorkspaceAuthorization>,
    pub(super) file_system: Option<Arc<dyn WorkspaceFileSystem>>,
    pub(super) workspace_folders: BTreeMap<String, WorkspaceAuthorization>,
    pub(super) folder_file_systems: BTreeMap<String, Arc<dyn WorkspaceFileSystem>>,
    pub(super) _file_system_watcher: Option<FileSystemWatcher>,
    pub(super) _folder_file_system_watchers: Vec<FileSystemWatcher>,
    pub(super) session_additional_directory_watchers:
        BTreeMap<(SessionId, PathBuf), SessionAdditionalDirectoryWatcher>,
    pub(super) _git_watcher: Option<GitWatcher>,
    pub(super) git: Option<Arc<GitRuntime>>,
    pub(super) workspace_search: Option<Arc<WorkspaceSearchService>>,
    pub(super) folder_workspace_search: BTreeMap<String, Arc<WorkspaceSearchService>>,
    pub(super) session_additional_directory_search:
        BTreeMap<(SessionId, PathBuf), Arc<WorkspaceSearchService>>,
    pub(super) ripgrep: Option<RipgrepExecutable>,
    pub(super) agent_grep: Option<Arc<AgentGrepService>>,
    pub(super) code_index: Option<Arc<CodeIndexRuntime>>,
    pub(super) symbol_index: Option<Arc<SymbolIndexRuntime>>,
    pub(super) code_index_semantic: Option<Arc<CodeIndexSemanticService>>,
    pub(super) code_index_semantic_job: Option<Arc<SemanticIndexJobController>>,
    pub(super) cloud_code_index: Option<Arc<CloudCodeIndexController>>,
    pub(super) _customizations: Option<Arc<WorkspaceCustomizations>>,
    pub(super) terminals: Option<Arc<crate::terminal_service::TerminalService>>,
    pub(super) folder_terminals: BTreeMap<String, Arc<crate::terminal_service::TerminalService>>,
    pub(super) debug_adapters: Option<Arc<crate::debug_service::DebugAdapterService>>,
    pub(super) folder_debug_adapters:
        BTreeMap<String, Arc<crate::debug_service::DebugAdapterService>>,
    pub(super) session_workspace_access: Arc<SessionWorkspaceAccess>,
    pub(super) turn_executor: TurnExecutor,
}

pub(super) struct SessionAdditionalDirectoryWatcher {
    workspace: zeta_workspace::TrustedWorkspace,
    _watcher: FileSystemWatcher,
}

pub(super) struct SessionAdditionalDirectorySnapshot {
    pub(super) root: PathBuf,
    pub(super) decision: WorkspaceTrustDecision,
    pub(super) permissions: zeta_workspace_access::AdditionalDirectoryPermissions,
}

pub(super) struct SessionAdditionalDirectorySnapshotSet {
    pub(super) revision: u64,
    pub(super) directories: Vec<SessionAdditionalDirectorySnapshot>,
}

impl WorkspaceRuntime {
    pub(super) fn empty(turn_executor: TurnExecutor) -> Self {
        Self {
            authorization: None,
            file_system: None,
            workspace_folders: BTreeMap::new(),
            folder_file_systems: BTreeMap::new(),
            _file_system_watcher: None,
            _folder_file_system_watchers: Vec::new(),
            session_additional_directory_watchers: BTreeMap::new(),
            _git_watcher: None,
            git: None,
            workspace_search: None,
            folder_workspace_search: BTreeMap::new(),
            session_additional_directory_search: BTreeMap::new(),
            ripgrep: None,
            agent_grep: None,
            code_index: None,
            symbol_index: None,
            code_index_semantic: None,
            code_index_semantic_job: None,
            cloud_code_index: None,
            _customizations: None,
            terminals: None,
            folder_terminals: BTreeMap::new(),
            debug_adapters: None,
            folder_debug_adapters: BTreeMap::new(),
            session_workspace_access: Arc::new(SessionWorkspaceAccess::default()),
            turn_executor,
        }
    }
}

pub(super) struct LocalWorkspaceHost {
    tools: Arc<WorkspaceToolPorts>,
    hooks: Arc<DeclarativeHookRuntime>,
    trust: WorkspaceSwitchTrustPolicy,
}

impl LocalWorkspaceHost {
    pub(super) fn replace_browser_host_available(
        &self,
        available: bool,
    ) -> Result<(), WorkspaceRuntimeError> {
        self.tools.replace_host_available(available)
    }

    pub(super) fn record_tool_reconcile_failure(&self, error: impl Into<String>) {
        self.tools.record_reconcile_failure(error);
    }
}

#[derive(Clone)]
pub(crate) struct WorkspaceRuntimeControl {
    authority_gate: Arc<Mutex<()>>,
    runtime: Arc<RwLock<WorkspaceRuntime>>,
    tools: Arc<WorkspaceToolPorts>,
    threads: Arc<ThreadController>,
    multi_agent: Arc<MultiAgentCoordinator>,
    sessions: Arc<SessionCoordinator>,
    turn_backend: Arc<dyn zeta_core::TurnExecutionBackend>,
    updates: Arc<super::update_broker::UpdateBroker>,
    hooks: Arc<DeclarativeHookRuntime>,
    mcp_status: Arc<RwLock<zeta_mcp_extension::McpRuntimeStatusSnapshot>>,
    config: Option<Arc<ConfigStore>>,
    local_tool_config: Arc<RwLock<LocalToolConfig>>,
    workspace_index_storage: Option<Arc<WorkspaceIndexStorage>>,
    fast_regex_worker_command: Option<zeta_fast_regex_search::FastRegexWorkerCommand>,
    code_index_semantic_models: Option<CodeIndexSemanticModels>,
    semantic_model_provider: Option<Arc<dyn SemanticModelProvider>>,
    extension_hosts: Option<super::extension_host_runtime::ExtensionHostRuntime>,
}

impl WorkspaceRuntimeControl {
    pub(crate) fn reconcile_local_tool_config(
        &self,
        config: &zeta_config::ResolvedConfig,
    ) -> Result<(), WorkspaceRuntimeError> {
        let _authority = self.authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let local_tool_config = LocalToolConfig::from_resolved(config);
        let (
            authorization,
            code_index,
            symbol_index,
            semantic,
            cloud,
            customizations,
            session_workspace_access,
            agent_grep,
        ) = {
            let runtime = self
                .runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                runtime.authorization.clone(),
                runtime.code_index.clone(),
                runtime.symbol_index.clone(),
                runtime.code_index_semantic.clone(),
                runtime.cloud_code_index.clone(),
                runtime._customizations.clone(),
                Arc::clone(&runtime.session_workspace_access),
                runtime.agent_grep.clone(),
            )
        };
        let Some(authorization) = authorization else {
            *self
                .local_tool_config
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = local_tool_config;
            return Ok(());
        };
        if authorization.decision() == WorkspaceTrustDecision::Restricted {
            *self
                .local_tool_config
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = local_tool_config;
            return Ok(());
        }
        let execution = authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let mut local = compose_local_tools_with_config(
            execution.clone(),
            &local_tool_config,
            session_workspace_access,
            agent_grep,
            self.workspace_index_storage.clone(),
            self.fast_regex_worker_command.as_ref(),
        )
        .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        if let Some(code_index) = code_index {
            let action_policy_revision = local.action_policy_revision().clone();
            local = append_local_tool(
                local,
                Arc::new(
                    CodeRetrievalTool::new(
                        execution,
                        code_index.index(),
                        symbol_index.map(|runtime| runtime.index()),
                        semantic,
                        cloud,
                    )
                    .with_action_policy_revision(action_policy_revision),
                ),
            );
        }
        local = append_multi_agent_tools(
            local,
            &self.multi_agent,
            &self.sessions,
            &self.turn_backend,
            customizations.as_ref(),
        );
        let agent_grep = Arc::clone(&local.agent_grep);
        let local_port = local
            .tool_port()
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        self.tools.replace_local(Some(local_port))?;
        self.runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .agent_grep = Some(agent_grep);
        *self
            .local_tool_config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = local_tool_config;
        Ok(())
    }

    pub(crate) fn reconcile_hooks(
        &self,
        config: &zeta_config::HooksConfig,
    ) -> Result<(), WorkspaceRuntimeError> {
        let _authority = self.authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        self.hooks.replace_config(config.clone());
        let workspace = self
            .runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()
            .filter(|authorization| authorization.decision() != WorkspaceTrustDecision::Restricted)
            .map(|authorization| authorization.root().clone());
        match workspace {
            Some(workspace) => self
                .hooks
                .bind_workspace(workspace)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string())),
            None => {
                self.hooks.unbind_workspace();
                Ok(())
            }
        }
    }

    pub(crate) fn replace_mcp_status(
        &self,
        snapshot: zeta_mcp_extension::McpRuntimeStatusSnapshot,
    ) {
        *self
            .mcp_status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = snapshot;
    }

    pub(crate) fn reconcile_semantic_code_index_runtime(
        &self,
    ) -> Result<(), WorkspaceRuntimeError> {
        let _authority = self.authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let (
            authorization,
            code_index,
            symbol_index,
            cloud,
            customizations,
            previous_watcher,
            previous_job,
        ) = {
            let mut runtime = self
                .runtime
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(authorization) = runtime.authorization.as_ref().cloned() else {
                return Ok(());
            };
            if authorization.decision() == WorkspaceTrustDecision::Restricted {
                return Ok(());
            }
            let Some(code_index) = runtime.code_index.clone() else {
                return Ok(());
            };
            let Some(symbol_index) = runtime.symbol_index.clone() else {
                return Ok(());
            };
            let Some(customizations) = runtime._customizations.clone() else {
                return Ok(());
            };
            let previous_watcher = runtime._file_system_watcher.take();
            let previous_job = runtime.code_index_semantic_job.take();
            runtime.code_index_semantic = None;
            (
                authorization,
                code_index,
                symbol_index,
                runtime.cloud_code_index.clone(),
                customizations,
                previous_watcher,
                previous_job,
            )
        };
        drop(previous_watcher);
        drop(previous_job);
        self.tools.replace_local(None)?;

        let semantic = open_code_index_semantic_runtime(
            &code_index,
            self.code_index_semantic_models.as_ref(),
            self.semantic_model_provider.as_ref(),
            self.config.as_ref(),
            self.workspace_index_storage.as_ref(),
        )?;
        let semantic_job = semantic.as_ref().and_then(|service| {
            match SemanticIndexJobController::start(Arc::clone(service)) {
                Ok(job) => Some(job),
                Err(error) => {
                    log::warn!("semantic code-index job is unavailable: {error}");
                    None
                }
            }
        });
        let execution = authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let local_tool_config = self
            .local_tool_config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let session_workspace_access = self
            .runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .session_workspace_access
            .clone();
        let local = compose_local_tools_with_config(
            execution.clone(),
            &local_tool_config,
            session_workspace_access,
            self.runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .agent_grep
                .clone(),
            self.workspace_index_storage.clone(),
            self.fast_regex_worker_command.as_ref(),
        )
        .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let action_policy_revision = local.action_policy_revision().clone();
        let local = append_local_tool(
            local,
            Arc::new(
                CodeRetrievalTool::new(
                    execution,
                    code_index.index(),
                    Some(symbol_index.index()),
                    semantic.clone(),
                    cloud.clone(),
                )
                .with_action_policy_revision(action_policy_revision),
            ),
        );
        let turn_backend: Arc<dyn zeta_core::TurnExecutionBackend> = self.turn_backend.clone();
        let local = append_multi_agent_tools(
            local,
            &self.multi_agent,
            &self.sessions,
            &turn_backend,
            Some(&customizations),
        );
        let watcher = FileSystemWatcher::start_with_observers(
            authorization.root().clone(),
            Arc::clone(&self.updates),
            Arc::clone(&code_index),
            Arc::clone(&symbol_index),
            semantic_job.clone(),
            customizations,
            Some(Arc::clone(&local.agent_grep)),
        )
        .map_err(|error| {
            WorkspaceRuntimeError::Failed(format!(
                "failed to rebind semantic code-index watcher: {error}"
            ))
        })?;
        let local_port = local
            .tool_port()
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        self.tools.replace_local(Some(local_port))?;
        let context_source = Arc::new(CodeRetrievalContextSource::new(
            code_index.index(),
            Some(symbol_index.index()),
            semantic.clone(),
            cloud.clone(),
            self.config.clone(),
            authorization.root().trust_id(),
        ));
        let mut runtime = self
            .runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        runtime.code_index_semantic = semantic;
        runtime.code_index_semantic_job = semantic_job;
        runtime._file_system_watcher = Some(watcher);
        runtime.turn_executor = runtime
            .turn_executor
            .clone()
            .with_context_source(context_source);
        Ok(())
    }

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
        let restricted_authorization =
            WorkspaceAuthorization::new(root.clone(), WorkspaceTrustDecision::Restricted);
        let repository_inspection = restricted_authorization
            .require(WorkspaceCapability::InspectRepository)
            .map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to authorize Git inspection".into())
            })?;
        let restricted_git = GitRuntime::new(repository_inspection, Arc::clone(&self.updates))
            .map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to initialize Git runtime".into())
            })?;
        if let Some(extension_hosts) = &self.extension_hosts {
            extension_hosts.unbind_workspace();
        }
        authorization.revoke();
        self.hooks.unbind_workspace();
        let folder_authorizations = std::mem::take(&mut runtime.workspace_folders);
        runtime.folder_file_systems.clear();
        let folder_workspace_search = std::mem::take(&mut runtime.folder_workspace_search);
        let folder_terminals = std::mem::take(&mut runtime.folder_terminals);
        let folder_debug_adapters = std::mem::take(&mut runtime.folder_debug_adapters);
        let cloud_code_index = runtime.cloud_code_index.clone();
        let old_file_system_watcher = runtime._file_system_watcher.take();
        let old_folder_file_system_watchers =
            std::mem::take(&mut runtime._folder_file_system_watchers);
        let code_index = runtime.code_index.clone();
        let symbol_index = runtime.symbol_index.clone();
        let customizations = runtime._customizations.clone();
        let terminals = runtime.terminals.take();
        let debug_adapters = runtime.debug_adapters.take();
        let search = runtime.workspace_search.take();
        let git = runtime.git.take();
        let git_watcher = runtime._git_watcher.take();
        runtime.cloud_code_index = None;
        runtime.code_index_semantic = None;
        runtime.code_index_semantic_job = None;
        runtime.authorization = Some(restricted_authorization);
        runtime.git = Some(Arc::clone(&restricted_git));
        drop(runtime);

        for (_, authorization) in folder_authorizations {
            authorization.revoke();
        }
        for (_, search) in folder_workspace_search {
            search.cancel_all();
        }
        for (_, terminals) in folder_terminals {
            terminals.terminate_all();
        }
        for (_, debug_adapters) in folder_debug_adapters {
            debug_adapters.terminate_all();
        }
        drop(old_file_system_watcher);
        drop(old_folder_file_system_watchers);
        let (restricted_watcher, watcher_error) = match (code_index, symbol_index, customizations) {
            (Some(code_index), Some(symbol_index), Some(customizations)) => {
                match FileSystemWatcher::start_with_observers(
                    root.clone(),
                    Arc::clone(&self.updates),
                    code_index,
                    symbol_index,
                    None,
                    customizations,
                    None,
                ) {
                    Ok(watcher) => (Some(watcher), None),
                    Err(error) => (None, Some(error)),
                }
            }
            _ => (None, None),
        };
        self.runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            ._file_system_watcher = restricted_watcher;

        let tool_result = self.tools.replace_executable(None, false);
        if let Some(terminals) = terminals {
            terminals.terminate_all();
        }
        if let Some(debug_adapters) = debug_adapters {
            debug_adapters.terminate_all();
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
        interrupt_result?;
        if let Some(error) = watcher_error {
            return Err(WorkspaceRuntimeError::Failed(format!(
                "failed to restrict filesystem watcher: {error}"
            )));
        }
        Ok(())
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
    dynamic: Option<ToolPort>,
    executables_enabled: bool,
    extension: Option<ToolPort>,
    host: Option<ToolPort>,
    host_available: bool,
    local: Option<ToolPort>,
    mcp: Option<ToolPort>,
    search: ToolSearchOptions,
    search_status: ToolSearchEmbeddingStatus,
    registry_generation: ToolRegistryGeneration,
}

pub(crate) struct WorkspaceToolPorts {
    state: Mutex<WorkspaceToolPortState>,
    reloadable: Arc<ReloadableToolPorts>,
    host: ToolPort,
    semantic_model_provider: Option<Arc<dyn SemanticModelProvider>>,
}

impl WorkspaceToolPorts {
    #[cfg(test)]
    pub(crate) fn definitions(&self) -> Vec<zeta_protocol::ToolDefinition> {
        self.reloadable.tools().definitions()
    }

    fn new(
        host: ToolPort,
        mcp: Option<ToolPort>,
        dynamic: Option<ToolPort>,
        extension: Option<ToolPort>,
        search_config: &ToolSearchConfig,
        providers: &std::collections::BTreeMap<ProviderId, ModelProviderConfig>,
        semantic_model_provider: Option<Arc<dyn SemanticModelProvider>>,
    ) -> Result<Arc<Self>, WorkspaceRuntimeError> {
        let search =
            resolve_tool_search(search_config, providers, semantic_model_provider.as_ref());
        let registry_generation = ToolRegistryGeneration::new(1);
        let ports = extension
            .iter()
            .chain(dynamic.iter())
            .chain(mcp.iter())
            .cloned()
            .collect();
        let combined = combine_tool_ports_at_generation_with_search(
            ports,
            registry_generation,
            search.options.clone(),
        )
        .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        Ok(Arc::new(Self {
            state: Mutex::new(WorkspaceToolPortState {
                dynamic,
                executables_enabled: false,
                extension,
                host: None,
                host_available: false,
                local: None,
                mcp,
                search: search.options,
                search_status: search.status,
                registry_generation,
            }),
            reloadable: ReloadableToolPorts::new(combined),
            host,
            semantic_model_provider,
        }))
    }

    fn replace_local(&self, local: Option<ToolPort>) -> Result<(), WorkspaceRuntimeError> {
        self.replace(|state| {
            state.local = local;
            Ok(())
        })
    }

    fn replace_executable(
        &self,
        local: Option<ToolPort>,
        executables_enabled: bool,
    ) -> Result<(), WorkspaceRuntimeError> {
        let host = self.host.clone();
        self.replace(|state| {
            state.executables_enabled = executables_enabled;
            state.host = (executables_enabled && state.host_available).then_some(host);
            state.local = local;
            Ok(())
        })
    }

    pub(crate) fn replace_host_available(
        &self,
        host_available: bool,
    ) -> Result<(), WorkspaceRuntimeError> {
        let host = self.host.clone();
        self.replace(|state| {
            state.host_available = host_available;
            state.host = (host_available && state.executables_enabled).then_some(host);
            Ok(())
        })
    }

    fn replace_dynamic(&self, dynamic: Option<ToolPort>) -> Result<(), WorkspaceRuntimeError> {
        self.replace(|state| {
            state.dynamic = dynamic;
            Ok(())
        })
    }

    fn replace_extension(&self, extension: Option<ToolPort>) -> Result<(), WorkspaceRuntimeError> {
        self.replace(|state| {
            state.extension = extension;
            Ok(())
        })
    }

    pub(crate) fn reconcile_user_config(
        &self,
        mcp: Option<ToolPort>,
        search_config: &ToolSearchConfig,
        providers: &std::collections::BTreeMap<ProviderId, ModelProviderConfig>,
    ) -> Result<(), WorkspaceRuntimeError> {
        let search = resolve_tool_search(
            search_config,
            providers,
            self.semantic_model_provider.as_ref(),
        );
        self.replace(|state| {
            state.mcp = mcp;
            state.search = search.options;
            state.search_status = search.status;
            Ok(())
        })
    }

    fn replace(
        &self,
        update: impl FnOnce(&mut WorkspaceToolPortState) -> Result<(), WorkspaceRuntimeError>,
    ) -> Result<(), WorkspaceRuntimeError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| WorkspaceRuntimeError::Failed("Workspace tool state poisoned".into()))?;
        let mut next = WorkspaceToolPortState {
            dynamic: state.dynamic.clone(),
            executables_enabled: state.executables_enabled,
            extension: state.extension.clone(),
            host: state.host.clone(),
            host_available: state.host_available,
            local: state.local.clone(),
            mcp: state.mcp.clone(),
            search: state.search.clone(),
            search_status: state.search_status.clone(),
            registry_generation: ToolRegistryGeneration::new(
                state
                    .registry_generation
                    .get()
                    .checked_add(1)
                    .ok_or_else(|| {
                        WorkspaceRuntimeError::Failed(
                            "Workspace tool registry generation overflow".into(),
                        )
                    })?,
            ),
        };
        update(&mut next)?;
        let ports = next
            .extension
            .iter()
            .chain(next.dynamic.iter())
            .chain(next.host.iter())
            .chain(next.local.iter())
            .chain(next.mcp.iter())
            .cloned()
            .collect();
        let combined = combine_tool_ports_at_generation_with_search(
            ports,
            next.registry_generation,
            next.search.clone(),
        )
        .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        *state = next;
        self.reloadable.replace(combined);
        Ok(())
    }

    pub(crate) fn record_reconcile_failure(&self, error: impl Into<String>) {
        self.reloadable.record_reconcile_failure(error);
    }

    pub(crate) fn tool_search_status(&self) -> ToolSearchEmbeddingStatus {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .search_status
            .clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceRuntimeError {
    Unavailable,
    Busy,
    AccessRevisionConflict,
    TrustRequired,
    Failed(String),
}

impl fmt::Display for WorkspaceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("local Workspace switching is unavailable"),
            Self::Busy => formatter.write_str("a Turn is still active in the current Workspace"),
            Self::AccessRevisionConflict => {
                formatter.write_str("the additional-directory permissions changed")
            }
            Self::TrustRequired => {
                formatter.write_str("the Workspace is not trusted for executable local services")
            }
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for WorkspaceRuntimeError {}

impl AppServer {
    pub(crate) fn with_extension_tool_port(
        mut self,
        extension: Option<ToolPort>,
    ) -> Result<Self, WorkspaceRuntimeError> {
        let Some(extension) = extension else {
            return Ok(self);
        };
        if self.extension_tool_port.is_some() {
            return Err(WorkspaceRuntimeError::Failed(
                "extension tools are already installed".into(),
            ));
        }
        if let Some(tools) = self
            .local_workspace_host
            .as_ref()
            .map(|host| Arc::clone(&host.tools))
        {
            tools.replace_extension(Some(extension.clone()))?;
        } else {
            let ports = std::iter::once(extension.clone())
                .chain(self.dynamic_tool_port.iter().cloned())
                .collect();
            let combined = combine_tool_ports_at_generation_with_search(
                ports,
                ToolRegistryGeneration::new(1),
                ToolSearchOptions::default(),
            )
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?
            .expect("one extension ToolPort produces a combined runtime");
            self = self.with_tool_service(combined.tools, combined.policy);
        }
        self.extension_tool_port = Some(extension);
        Ok(self)
    }

    /// Installs client-hosted tools that execute through durable agent interactions.
    ///
    /// The definitions are validated and frozen before they enter the shared registry. A live
    /// connection must separately declare `DynamicTool` interaction support, list the exact hosted
    /// tool name, and subscribe to the target Thread before App Server can route an authorized
    /// invocation to it.
    pub fn with_dynamic_tools(
        mut self,
        specifications: Vec<zeta_protocol::DynamicToolSpec>,
    ) -> Result<Self, DynamicToolCompositionError> {
        if self.dynamic_tool_port.is_some() {
            return Err(DynamicToolCompositionError::configuration(
                "dynamic tools are already installed",
            ));
        }
        let Some(composition) = compose_dynamic_tools(specifications)? else {
            return Ok(self);
        };
        let port = ToolPort::dynamic(composition.tools, composition.policy);
        if let Some(tools) = self
            .local_workspace_host
            .as_ref()
            .map(|host| Arc::clone(&host.tools))
        {
            tools
                .replace_dynamic(Some(port.clone()))
                .map_err(|error| DynamicToolCompositionError::configuration(error.to_string()))?;
        } else {
            let ports = self
                .extension_tool_port
                .iter()
                .cloned()
                .chain(std::iter::once(port.clone()))
                .collect();
            let combined = combine_tool_ports_at_generation_with_search(
                ports,
                ToolRegistryGeneration::new(1),
                ToolSearchOptions::default(),
            )
            .map_err(|error| DynamicToolCompositionError::configuration(error.to_string()))?
            .expect("one dynamic ToolPort produces a combined runtime");
            self = self.with_tool_service(combined.tools, combined.policy);
        }
        self.dynamic_tool_port = Some(port);
        Ok(self)
    }

    pub(super) fn tool_search_embedding_status(&self) -> ToolSearchEmbeddingStatus {
        self.local_workspace_host
            .as_ref()
            .map(|host| host.tools.tool_search_status())
            .unwrap_or(ToolSearchEmbeddingStatus::Disabled)
    }

    pub(super) fn active_workspace_trust_id(&self) -> Option<zeta_workspace::WorkspaceTrustId> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()
            .map(|authorization| authorization.root().trust_id())
    }

    pub(super) fn active_workspace_binding(&self) -> Option<zeta_workspace::WorkspaceBinding> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()
            .map(|authorization| zeta_workspace::WorkspaceBinding::from_root(authorization.root()))
    }

    pub(super) fn reconcile_semantic_code_index_runtime(
        &self,
    ) -> Result<(), WorkspaceRuntimeError> {
        let Some(control) = self.workspace_runtime_control() else {
            return Ok(());
        };
        control.reconcile_semantic_code_index_runtime()
    }

    pub(super) fn validate_semantic_code_index_selection(
        &self,
    ) -> Result<(), WorkspaceRuntimeError> {
        if self.code_index_semantic_models.is_some() {
            return Ok(());
        }
        let snapshot = self
            .config
            .as_ref()
            .ok_or(WorkspaceRuntimeError::Unavailable)?
            .read_snapshot()
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let models = snapshot
            .values
            .semantic_code_index
            .selection
            .remote_models()
            .ok_or_else(|| {
                WorkspaceRuntimeError::Failed(
                    "semantic code-index models are not configured".into(),
                )
            })?;
        let provider = self.semantic_model_provider.as_ref().ok_or_else(|| {
            WorkspaceRuntimeError::Failed("semantic model provider runtime is not installed".into())
        })?;
        resolve_semantic_model_invokers(provider, models, &snapshot.values.providers)
            .map(|_| ())
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))
    }

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
        let (search_config, providers, hook_config) = match &self.config {
            Some(config) => {
                let snapshot = config
                    .read_snapshot()
                    .map_err(|error| WorkspaceRuntimeError::Failed(error.0))?;
                (
                    snapshot.values.tool_search,
                    snapshot.values.providers,
                    snapshot.values.hooks,
                )
            }
            None => (
                ToolSearchConfig::default(),
                Default::default(),
                Default::default(),
            ),
        };
        let tools = WorkspaceToolPorts::new(
            self.browser_tool_port.clone(),
            mcp,
            self.dynamic_tool_port.clone(),
            self.extension_tool_port.clone(),
            &search_config,
            &providers,
            self.semantic_model_provider.clone(),
        )?;
        let policy = Arc::new(ApprovalModeActionPolicyService::new(
            tools.reloadable.policy(),
            self.approval_review_model.clone(),
        ));
        let hooks = Arc::new(DeclarativeHookRuntime::new(
            hook_config,
            tools.reloadable.policy(),
        ));
        let mut executor = TurnExecutor::new(
            self.sessions.threads().clone(),
            Arc::clone(&self.model),
            tools.reloadable.tools(),
            policy,
        )
        .with_hooks(hooks.clone())
        .with_thread_updates(Arc::new(AppServerThreadUpdates {
            sessions: Arc::clone(&self.sessions),
            updates: Arc::clone(&self.updates),
        }));
        executor = executor.with_extensions(Arc::clone(&self.agent_extensions));
        self.turn_backend.install_executor(executor.clone());
        self.workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .turn_executor = executor;
        self.local_workspace_host = Some(LocalWorkspaceHost {
            tools,
            hooks,
            trust,
        });
        self.use_current_local_turn_backend();
        Ok(self)
    }

    pub(crate) fn with_local_tool_config(self, config: LocalToolConfig) -> Self {
        *self
            .local_tool_config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = config;
        self
    }

    pub(crate) fn local_workspace_tool_ports(&self) -> Option<Arc<WorkspaceToolPorts>> {
        self.local_workspace_host
            .as_ref()
            .map(|host| Arc::clone(&host.tools))
    }

    pub(crate) fn local_hook_runtime(&self) -> Option<Arc<DeclarativeHookRuntime>> {
        self.local_workspace_host
            .as_ref()
            .map(|host| Arc::clone(&host.hooks))
    }

    pub(crate) fn workspace_runtime_control(&self) -> Option<WorkspaceRuntimeControl> {
        self.local_workspace_host
            .as_ref()
            .map(|host| WorkspaceRuntimeControl {
                authority_gate: Arc::clone(&self.workspace_authority_gate),
                runtime: Arc::clone(&self.workspace_runtime),
                tools: Arc::clone(&host.tools),
                threads: self.sessions.threads().clone(),
                multi_agent: Arc::clone(&self.multi_agent),
                sessions: Arc::clone(&self.sessions),
                turn_backend: self.turn_backend.clone(),
                updates: Arc::clone(&self.updates),
                hooks: Arc::clone(&host.hooks),
                mcp_status: Arc::clone(&self.mcp_status),
                config: self.config.clone(),
                local_tool_config: Arc::clone(&self.local_tool_config),
                workspace_index_storage: self.workspace_index_storage.clone(),
                fast_regex_worker_command: self.fast_regex_worker_command.clone(),
                code_index_semantic_models: self.code_index_semantic_models.clone(),
                semantic_model_provider: self.semantic_model_provider.clone(),
                extension_hosts: self.extension_hosts.clone(),
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

    pub(crate) fn switch_local_workspace_root_with_decision(
        &self,
        root: PathBuf,
        decision: WorkspaceTrustDecision,
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
        self.activate_local_workspace(WorkspaceAuthorization::new(workspace, decision), host)
    }

    pub(crate) fn authorize_local_workspace_root(
        &self,
        root: PathBuf,
        decision: Option<WorkspaceTrustDecision>,
    ) -> Result<WorkspaceAuthorization, WorkspaceRuntimeError> {
        let host = self
            .local_workspace_host
            .as_ref()
            .ok_or(WorkspaceRuntimeError::Unavailable)?;
        let workspace = WorkspaceRoot::open(root)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        match decision {
            Some(decision) => Ok(WorkspaceAuthorization::new(workspace, decision)),
            None => host.trust.authorize(workspace),
        }
    }

    pub(crate) fn activate_local_workspace_folders(
        &self,
        folders: Vec<(String, WorkspaceAuthorization)>,
    ) -> Result<Vec<(String, PathBuf, WorkspaceTrustDecision)>, WorkspaceRuntimeError> {
        let host = self
            .local_workspace_host
            .as_ref()
            .ok_or(WorkspaceRuntimeError::Unavailable)?;
        let _workspace_authority = self.workspace_authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        self.ensure_workspace_switch_is_idle()?;
        let Some((_, primary)) = folders.first() else {
            return Err(WorkspaceRuntimeError::Failed(
                "Workspace must contain at least one folder".into(),
            ));
        };
        let workspaces = folders
            .iter()
            .map(|(id, authorization)| {
                let capability = if authorization.decision() == WorkspaceTrustDecision::Restricted {
                    WorkspaceCapability::InspectRepository
                } else {
                    WorkspaceCapability::MutateRepository
                };
                authorization
                    .require(capability)
                    .map(|workspace| (id.clone(), workspace))
                    .map_err(|_| WorkspaceRuntimeError::TrustRequired)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let git = GitRuntime::new_for_workspace_folders(workspaces, Arc::clone(&self.updates))
            .map_err(|error| {
                WorkspaceRuntimeError::Failed(format!(
                    "failed to initialize multi-root Git runtime: {error:?}"
                ))
            })?;
        let watcher = git.start_watching();
        let workspace_folders = folders.iter().cloned().collect::<BTreeMap<_, _>>();
        let folder_file_systems = folders
            .iter()
            .map(|(id, authorization)| {
                let file_system: Arc<dyn WorkspaceFileSystem> =
                    Arc::new(LocalFileSystem::new(authorization.root().clone()));
                (id.clone(), file_system)
            })
            .collect::<BTreeMap<_, _>>();
        self.activate_local_workspace(primary.clone(), host)?;
        let (ripgrep, agent_grep, primary_search, primary_terminals, primary_debug_adapters) = {
            let runtime = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                runtime.ripgrep.clone(),
                runtime.agent_grep.clone(),
                runtime.workspace_search.clone(),
                runtime.terminals.clone(),
                runtime.debug_adapters.clone(),
            )
        };
        let folder_file_system_watchers = folders
            .iter()
            .skip(1)
            .map(|(id, authorization)| {
                FileSystemWatcher::start_for_workspace_folder(
                    authorization.root().clone(),
                    Arc::clone(&self.updates),
                    id.clone(),
                    agent_grep.clone(),
                )
                .map_err(|error| {
                    WorkspaceRuntimeError::Failed(format!(
                        "failed to initialize workspace folder watcher: {error}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let folder_workspace_search = folders
            .iter()
            .enumerate()
            .filter_map(|(index, (id, authorization))| {
                if authorization.decision() == WorkspaceTrustDecision::Restricted {
                    return None;
                }
                let service = if index == 0 {
                    primary_search.clone()
                } else {
                    ripgrep.as_ref().map(|ripgrep| {
                        Arc::new(WorkspaceSearchService::new(
                            authorization.root().clone(),
                            ripgrep.clone(),
                        ))
                    })
                }?;
                Some((id.clone(), service))
            })
            .collect::<BTreeMap<_, _>>();
        let mut folder_terminals = BTreeMap::new();
        for (index, (id, authorization)) in folders.iter().enumerate() {
            if authorization.decision() == WorkspaceTrustDecision::Restricted {
                continue;
            }
            let terminals = if index == 0 {
                primary_terminals.clone()
            } else {
                let capability = authorization
                    .require(WorkspaceCapability::ExecuteProcess)
                    .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
                Some(Arc::new(
                    crate::terminal_service::TerminalService::new(capability).map_err(|_| {
                        WorkspaceRuntimeError::Failed(
                            "failed to initialize workspace folder terminal runtime".into(),
                        )
                    })?,
                ))
            };
            if let Some(terminals) = terminals {
                folder_terminals.insert(id.clone(), terminals);
            }
        }
        let mut folder_debug_adapters = BTreeMap::new();
        for (index, (id, authorization)) in folders.iter().enumerate() {
            if authorization.decision() == WorkspaceTrustDecision::Restricted {
                continue;
            }
            let debug_adapters = if index == 0 {
                primary_debug_adapters.clone()
            } else {
                Some(Arc::new(
                    crate::debug_service::DebugAdapterService::new(
                        authorization
                            .require(WorkspaceCapability::LoadExecutableConfiguration)
                            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?,
                        authorization
                            .require(WorkspaceCapability::ExecuteProcess)
                            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?,
                        crate::terminal_environment::safe_process_environment(),
                    )
                    .map_err(|_| {
                        WorkspaceRuntimeError::Failed(
                            "failed to initialize workspace folder debug adapter runtime".into(),
                        )
                    })?,
                ))
            };
            if let Some(debug_adapters) = debug_adapters {
                folder_debug_adapters.insert(id.clone(), debug_adapters);
            }
        }
        let (previous_watcher, previous_folder_watchers) = {
            let mut runtime = self
                .workspace_runtime
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous_watcher = runtime._git_watcher.take();
            let previous_folder_watchers =
                std::mem::take(&mut runtime._folder_file_system_watchers);
            runtime.workspace_folders = workspace_folders;
            runtime.folder_file_systems = folder_file_systems;
            runtime._folder_file_system_watchers = folder_file_system_watchers;
            runtime.folder_workspace_search = folder_workspace_search;
            runtime.folder_terminals = folder_terminals;
            runtime.folder_debug_adapters = folder_debug_adapters;
            runtime.git = Some(git);
            runtime._git_watcher = Some(watcher);
            (previous_watcher, previous_folder_watchers)
        };
        drop(previous_watcher);
        drop(previous_folder_watchers);
        self.reset_language_workspace_runtimes();
        Ok(folders
            .into_iter()
            .map(|(id, authorization)| {
                (
                    id,
                    authorization.root().canonical_path().to_path_buf(),
                    authorization.decision(),
                )
            })
            .collect())
    }

    pub(super) fn list_session_additional_directories(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionAdditionalDirectorySnapshotSet, WorkspaceRuntimeError> {
        self.sessions
            .read_session(session_id)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if runtime.authorization.is_none() {
            return Err(WorkspaceRuntimeError::Unavailable);
        }
        Ok(session_additional_directory_snapshots(
            &runtime.session_workspace_access,
            session_id,
        ))
    }

    pub(super) fn add_session_additional_directory(
        &self,
        session_id: &SessionId,
        root: PathBuf,
        permissions: zeta_workspace_access::AdditionalDirectoryPermissions,
    ) -> Result<
        (
            WorkspaceAccessMutation,
            SessionAdditionalDirectorySnapshotSet,
        ),
        WorkspaceRuntimeError,
    > {
        self.sessions
            .read_session(session_id)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let _workspace_authority = self.workspace_authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let (primary, session_workspace_access) = {
            let runtime = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let primary = runtime
                .authorization
                .as_ref()
                .map(|authorization| authorization.root().clone())
                .ok_or(WorkspaceRuntimeError::Unavailable)?;
            (primary, Arc::clone(&runtime.session_workspace_access))
        };
        let requested = if root.is_absolute() {
            root
        } else {
            primary.canonical_path().join(root)
        };
        let authorization = self.authorize_local_workspace_root(
            requested,
            Some(WorkspaceTrustDecision::Trusted(
                zeta_workspace::WorkspaceTrustSource::ExplicitUserDecision,
            )),
        )?;
        authorization
            .require(WorkspaceCapability::MutateRepository)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let mutation = session_workspace_access
            .add_directory(session_id.clone(), primary, authorization, permissions)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        self.reconcile_session_additional_directory_consumers(session_id)?;
        let snapshots =
            session_additional_directory_snapshots(&session_workspace_access, session_id);
        Ok((mutation, snapshots))
    }

    pub(super) fn remove_session_additional_directory(
        &self,
        session_id: &SessionId,
        root: &std::path::Path,
    ) -> Result<
        (
            WorkspaceAccessMutation,
            SessionAdditionalDirectorySnapshotSet,
        ),
        WorkspaceRuntimeError,
    > {
        self.sessions
            .read_session(session_id)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let _workspace_authority = self.workspace_authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let session_workspace_access = {
            let runtime = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if runtime.authorization.is_none() {
                return Err(WorkspaceRuntimeError::Unavailable);
            }
            Arc::clone(&runtime.session_workspace_access)
        };
        let mutation = session_workspace_access.remove_directory(session_id, root);
        self.reconcile_session_additional_directory_consumers(session_id)?;
        self.terminate_revoked_terminal_sessions();
        let snapshots =
            session_additional_directory_snapshots(&session_workspace_access, session_id);
        Ok((mutation, snapshots))
    }

    pub(super) fn set_session_additional_directory_permissions(
        &self,
        session_id: &SessionId,
        root: &std::path::Path,
        expected_revision: u64,
        permissions: zeta_workspace_access::AdditionalDirectoryPermissions,
    ) -> Result<
        (
            WorkspaceAccessMutation,
            SessionAdditionalDirectorySnapshotSet,
        ),
        WorkspaceRuntimeError,
    > {
        self.sessions
            .read_session(session_id)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let _workspace_authority = self.workspace_authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let session_workspace_access = {
            let runtime = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if runtime.authorization.is_none() {
                return Err(WorkspaceRuntimeError::Unavailable);
            }
            Arc::clone(&runtime.session_workspace_access)
        };
        let mutation = session_workspace_access
            .set_permissions(session_id, root, expected_revision, permissions)
            .map_err(|error| match error {
                zeta_workspace_access::WorkspaceAccessError::RevisionConflict { .. } => {
                    WorkspaceRuntimeError::AccessRevisionConflict
                }
                other => WorkspaceRuntimeError::Failed(other.to_string()),
            })?;
        self.reconcile_session_additional_directory_consumers(session_id)?;
        self.terminate_revoked_terminal_sessions();
        let snapshots =
            session_additional_directory_snapshots(&session_workspace_access, session_id);
        Ok((mutation, snapshots))
    }

    fn reconcile_session_additional_directory_consumers(
        &self,
        session_id: &SessionId,
    ) -> Result<(), WorkspaceRuntimeError> {
        let (access, customizations) = {
            let runtime = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                Arc::clone(&runtime.session_workspace_access),
                runtime._customizations.clone(),
            )
        };
        let load_workspaces = access
            .snapshot_for(session_id, WorkspaceCapability::LoadExecutableConfiguration)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?
            .map(|snapshot| snapshot.additional_roots().to_vec())
            .unwrap_or_default();
        let watch_workspaces = access
            .snapshot_for(session_id, WorkspaceCapability::ObserveFileChanges)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?
            .map(|snapshot| snapshot.additional_roots().to_vec())
            .unwrap_or_default();
        if let Some(host) = &self.local_workspace_host {
            let hook_workspaces = access
                .snapshot_for(session_id, WorkspaceCapability::DiscoverHooks)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?
                .into_iter()
                .flat_map(|snapshot| snapshot.additional_roots().to_vec())
                .filter_map(|discovery| {
                    access
                        .workspace_for(
                            session_id,
                            discovery.root().canonical_path(),
                            WorkspaceCapability::ExecuteProcess,
                        )
                        .transpose()
                        .map(|execution| execution.map(|execution| (discovery, execution)))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?
                .into_iter()
                .map(|(discovery, execution)| {
                    read_additional_workspace_config(discovery.root())
                        .map(|document| (document.hooks, discovery, execution))
                })
                .collect::<Result<Vec<_>, _>>()?;
            host.hooks
                .replace_session_workspaces(session_id.clone(), hook_workspaces)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        }
        let mut language_roots = access.roots_for(WorkspaceCapability::UseLanguageServices);
        let execution_roots = access.roots_for(WorkspaceCapability::ExecuteProcess);
        language_roots.retain(|root| execution_roots.contains(root));
        self.language
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain_workspace_roots(&language_roots);
        if let Some(customizations) = &customizations {
            customizations.reconcile_session(session_id, load_workspaces);
        }
        let desired = watch_workspaces
            .into_iter()
            .map(|workspace| (workspace.root().canonical_path().to_path_buf(), workspace))
            .collect::<BTreeMap<_, _>>();
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        runtime
            .session_additional_directory_watchers
            .retain(|(candidate_session, root), _| {
                candidate_session != session_id || desired.contains_key(root)
            });
        runtime
            .session_additional_directory_search
            .retain(|(candidate_session, root), _| {
                if candidate_session != session_id {
                    return true;
                }
                access
                    .workspace_for(
                        session_id,
                        root,
                        WorkspaceCapability::SearchRepositoryContent,
                    )
                    .ok()
                    .flatten()
                    .is_some()
            });
        let agent_grep = runtime.agent_grep.clone();
        let Some(customizations) = customizations else {
            return Ok(());
        };
        for (root, workspace) in desired {
            let key = (session_id.clone(), root);
            if let Some(existing) = runtime.session_additional_directory_watchers.get_mut(&key) {
                existing.workspace = workspace;
                continue;
            }
            let watcher = FileSystemWatcher::start_for_session_directory(
                workspace.root().clone(),
                Arc::clone(&self.updates),
                session_id.clone(),
                customizations.clone(),
                agent_grep.clone(),
            )
            .map_err(|error| {
                WorkspaceRuntimeError::Failed(format!(
                    "failed to initialize additional-directory watcher: {error}"
                ))
            })?;
            runtime.session_additional_directory_watchers.insert(
                key,
                SessionAdditionalDirectoryWatcher {
                    workspace,
                    _watcher: watcher,
                },
            );
        }
        Ok(())
    }

    pub(super) fn clear_session_additional_directories(&self, session_id: &SessionId) {
        let _workspace_authority = self
            .workspace_authority_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        runtime.session_workspace_access.clear_session(session_id);
        runtime
            .session_additional_directory_watchers
            .retain(|(candidate, _), _| candidate != session_id);
        runtime
            .session_additional_directory_search
            .retain(|(candidate, _), _| candidate != session_id);
        if let Some(customizations) = &runtime._customizations {
            customizations.remove_session(session_id);
        }
        if let Some(host) = &self.local_workspace_host {
            host.hooks.remove_session(session_id);
        }
        drop(runtime);
        self.terminate_revoked_terminal_sessions();
    }

    pub(crate) fn active_workspace_is_trusted(&self) -> bool {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()
            .is_some_and(|authorization| {
                authorization.decision() != WorkspaceTrustDecision::Restricted
            })
    }

    fn activate_local_workspace(
        &self,
        authorization: WorkspaceAuthorization,
        host: &LocalWorkspaceHost,
    ) -> Result<PathBuf, WorkspaceRuntimeError> {
        if self.workspace_authority_is_current(&authorization) {
            return Ok(authorization.root().canonical_path().to_path_buf());
        }
        let result = if authorization.decision() == WorkspaceTrustDecision::Restricted {
            self.commit_restricted_workspace_runtime(authorization, host)
        } else {
            let execution = authorization
                .require(WorkspaceCapability::ExecuteProcess)
                .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
            let local_tool_config = self
                .local_tool_config
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            let session_workspace_access = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .session_workspace_access
                .clone();
            let local = compose_local_tools_with_config(
                execution,
                &local_tool_config,
                session_workspace_access,
                None,
                self.workspace_index_storage.clone(),
                self.fast_regex_worker_command.as_ref(),
            )
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
            self.commit_trusted_workspace_runtime(authorization, local, host)
        };
        if result.is_ok() {
            self.reset_language_workspace_runtimes();
        }
        result
    }

    fn reset_language_workspace_runtimes(&self) {
        if let Ok(mut language) = self.language.lock() {
            language.reset_workspace();
        }
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
        let symbol_index = self.open_symbol_index_runtime(&code_index)?;
        self.retry_persisted_cloud_index_deletion(&code_index);
        let repository_inspection = authorization
            .require(WorkspaceCapability::InspectRepository)
            .map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to authorize Git inspection".into())
            })?;
        let git =
            GitRuntime::new(repository_inspection, Arc::clone(&self.updates)).map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to initialize Git runtime".into())
            })?;
        let session_workspace_access = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .session_workspace_access
            .clone();
        let customizations =
            WorkspaceCustomizations::discover(&canonical_root, session_workspace_access)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let file_system_watcher = FileSystemWatcher::start_with_observers(
            workspace,
            Arc::clone(&self.updates),
            Arc::clone(&code_index),
            Arc::clone(&symbol_index),
            None,
            customizations.clone(),
            None,
        )
        .map_err(|error| {
            WorkspaceRuntimeError::Failed(format!(
                "failed to initialize filesystem watcher: {error}"
            ))
        })?;

        host.hooks.unbind_workspace();
        host.tools.replace_executable(None, false)?;
        self.bind_workspace_skills(&canonical_root)?;
        if let Some(extension_hosts) = &self.extension_hosts {
            extension_hosts.unbind_workspace();
        }
        let mut current = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        current.session_workspace_access.clear();
        let next = WorkspaceRuntime {
            authorization: Some(authorization),
            file_system: Some(file_system),
            workspace_folders: BTreeMap::new(),
            folder_file_systems: BTreeMap::new(),
            _file_system_watcher: Some(file_system_watcher),
            _folder_file_system_watchers: Vec::new(),
            session_additional_directory_watchers: BTreeMap::new(),
            _git_watcher: None,
            git: Some(Arc::clone(&git)),
            workspace_search: None,
            folder_workspace_search: BTreeMap::new(),
            session_additional_directory_search: BTreeMap::new(),
            ripgrep: None,
            agent_grep: None,
            code_index: Some(code_index),
            symbol_index: Some(symbol_index),
            code_index_semantic: None,
            code_index_semantic_job: None,
            cloud_code_index: None,
            _customizations: Some(Arc::clone(&customizations)),
            terminals: None,
            folder_terminals: BTreeMap::new(),
            debug_adapters: None,
            folder_debug_adapters: BTreeMap::new(),
            session_workspace_access: Arc::clone(&current.session_workspace_access),
            turn_executor: current
                .turn_executor
                .clone()
                .with_harness_context_provider(customizations)
                .with_context_source(Arc::new(zeta_core::NoContextSource)),
        };
        let previous = std::mem::replace(&mut *current, next);
        drop(current);
        retire_workspace_runtime(previous, None, None, None);
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
        let symbol_index = self.open_symbol_index_runtime(&code_index)?;
        let code_index_semantic = self.open_code_index_semantic(&code_index)?;
        let code_index_semantic_job =
            code_index_semantic.as_ref().and_then(
                |service| match SemanticIndexJobController::start(Arc::clone(service)) {
                    Ok(job) => Some(job),
                    Err(error) => {
                        log::warn!("semantic code-index job is unavailable: {error}");
                        None
                    }
                },
            );
        let cloud_code_index = self.open_cloud_code_index_controller(&code_index);
        let canonical_root = workspace.canonical_path().to_path_buf();
        let session_workspace_access = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .session_workspace_access
            .clone();
        let customizations =
            WorkspaceCustomizations::discover(&canonical_root, session_workspace_access)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let repository_mutation = authorization
            .require(WorkspaceCapability::MutateRepository)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let git =
            GitRuntime::new(repository_mutation, Arc::clone(&self.updates)).map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to initialize Git runtime".into())
            })?;
        let agent_grep = Arc::clone(&local.agent_grep);
        let file_system_watcher = FileSystemWatcher::start_with_observers(
            workspace.clone(),
            Arc::clone(&self.updates),
            Arc::clone(&code_index),
            Arc::clone(&symbol_index),
            code_index_semantic_job.clone(),
            customizations.clone(),
            Some(Arc::clone(&agent_grep)),
        )
        .map_err(|error| {
            WorkspaceRuntimeError::Failed(format!(
                "failed to initialize filesystem watcher: {error}"
            ))
        })?;
        let retrieval_workspace = authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let action_policy_revision = local.action_policy_revision().clone();
        let local = append_local_tool(
            local,
            Arc::new(
                CodeRetrievalTool::new(
                    retrieval_workspace,
                    code_index.index(),
                    Some(symbol_index.index()),
                    code_index_semantic.clone(),
                    cloud_code_index.clone(),
                )
                .with_action_policy_revision(action_policy_revision),
            ),
        );
        let turn_backend: Arc<dyn zeta_core::TurnExecutionBackend> = self.turn_backend.clone();
        let local = append_multi_agent_tools(
            local,
            &self.multi_agent,
            &self.sessions,
            &turn_backend,
            Some(&customizations),
        );
        let local_port = local
            .tool_port()
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let (existing_search, existing_terminals) = {
            let current = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (current.workspace_search.clone(), current.terminals.clone())
        };
        let workspace_search = existing_search.unwrap_or_else(|| {
            Arc::new(WorkspaceSearchService::new(
                workspace.clone(),
                local.ripgrep.clone(),
            ))
        });
        let ripgrep = local.ripgrep.clone();
        workspace_search.cancel_all();
        workspace_search.switch_workspace(workspace.clone());
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
        let debug_adapters = Arc::new(
            crate::debug_service::DebugAdapterService::new(
                authorization
                    .require(WorkspaceCapability::LoadExecutableConfiguration)
                    .map_err(|_| WorkspaceRuntimeError::TrustRequired)?,
                authorization
                    .require(WorkspaceCapability::ExecuteProcess)
                    .map_err(|_| WorkspaceRuntimeError::TrustRequired)?,
                crate::terminal_environment::safe_process_environment(),
            )
            .map_err(|_| {
                WorkspaceRuntimeError::Failed("failed to initialize debug adapter runtime".into())
            })?,
        );
        host.hooks
            .bind_workspace(workspace.clone())
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        customizations.bind_hooks(Arc::clone(&host.hooks));
        host.tools.replace_executable(Some(local_port), true)?;
        self.bind_workspace_skills(&canonical_root)?;
        let context_source = Arc::new(CodeRetrievalContextSource::new(
            code_index.index(),
            Some(symbol_index.index()),
            code_index_semantic.clone(),
            cloud_code_index.clone(),
            self.config.clone(),
            workspace.trust_id(),
        ));
        let extension_workspace = authorization
            .require(WorkspaceCapability::ActivateWorkspaceExtension)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        if let Some(extension_hosts) = &self.extension_hosts {
            extension_hosts.unbind_workspace();
        }
        let mut current = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        current.session_workspace_access.clear();
        let next = WorkspaceRuntime {
            authorization: Some(authorization),
            file_system: Some(file_system),
            workspace_folders: BTreeMap::new(),
            folder_file_systems: BTreeMap::new(),
            _file_system_watcher: Some(file_system_watcher),
            _folder_file_system_watchers: Vec::new(),
            session_additional_directory_watchers: BTreeMap::new(),
            _git_watcher: None,
            git: Some(Arc::clone(&git)),
            workspace_search: Some(Arc::clone(&workspace_search)),
            folder_workspace_search: BTreeMap::new(),
            session_additional_directory_search: BTreeMap::new(),
            ripgrep: Some(ripgrep),
            agent_grep: Some(agent_grep),
            code_index: Some(code_index),
            symbol_index: Some(symbol_index),
            code_index_semantic,
            code_index_semantic_job,
            cloud_code_index,
            _customizations: Some(Arc::clone(&customizations)),
            terminals: Some(Arc::clone(&terminals)),
            folder_terminals: BTreeMap::new(),
            debug_adapters: Some(Arc::clone(&debug_adapters)),
            folder_debug_adapters: BTreeMap::new(),
            session_workspace_access: Arc::clone(&current.session_workspace_access),
            turn_executor: current
                .turn_executor
                .clone()
                .with_harness_context_provider(customizations)
                .with_context_source(context_source),
        };
        let previous = std::mem::replace(&mut *current, next);
        drop(current);
        retire_workspace_runtime(
            previous,
            Some(&workspace_search),
            Some(&terminals),
            Some(&debug_adapters),
        );
        if let Some(extension_hosts) = &self.extension_hosts
            && extension_hosts.bind_workspace(extension_workspace).is_err()
        {
            log::warn!("failed to bind executable Editor Extensions to the new workspace");
        }
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

    pub(super) fn workspace_features(&self) -> (bool, bool, bool, bool, bool, bool, bool) {
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
            switchable || runtime.debug_adapters.is_some(),
        )
    }

    pub(super) fn trusted_extension_workspace(&self) -> Option<zeta_workspace::TrustedWorkspace> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()?
            .require(WorkspaceCapability::ActivateWorkspaceExtension)
            .ok()
    }

    fn open_code_index_runtime(
        &self,
        workspace: WorkspaceRoot,
    ) -> Result<Arc<CodeIndexRuntime>, WorkspaceRuntimeError> {
        let trust_id = workspace.trust_id();
        if let Some(index_storage) = &self.workspace_index_storage {
            let lease = index_storage
                .acquire(&trust_id, WorkspaceIndexKind::Lexical)
                .map_err(|error| {
                    WorkspaceRuntimeError::Failed(format!(
                        "failed to lock lexical index storage: {error}"
                    ))
                })?;
            let storage = CodeIndexStorage::Persistent(lease.directory().join("index.sqlite3"));
            CodeIndexRuntime::open_with_lease(workspace, storage, lease).map_err(|error| {
                WorkspaceRuntimeError::Failed(format!("failed to open code index: {error}"))
            })
        } else {
            CodeIndexRuntime::open(workspace, CodeIndexStorage::Memory).map_err(|error| {
                WorkspaceRuntimeError::Failed(format!("failed to open code index: {error}"))
            })
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

    pub(super) fn agent_grep_index_context(
        &self,
    ) -> Result<(Arc<AgentGrepService>, WorkspaceRoot), RpcError> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = runtime
            .authorization
            .as_ref()
            .map(|authorization| authorization.root().clone())
            .ok_or_else(|| RpcError::new(-32090, AppServerErrorName::CodeIndexUnavailable))?;
        let service = runtime
            .agent_grep
            .clone()
            .ok_or_else(|| RpcError::new(-32090, AppServerErrorName::CodeIndexUnavailable))?;
        Ok((service, root))
    }

    fn open_symbol_index_runtime(
        &self,
        code_index: &Arc<CodeIndexRuntime>,
    ) -> Result<Arc<SymbolIndexRuntime>, WorkspaceRuntimeError> {
        let trust_id = code_index.root().trust_id();
        if let Some(index_storage) = &self.workspace_index_storage {
            let lease = index_storage
                .acquire(&trust_id, WorkspaceIndexKind::Symbols)
                .map_err(|error| {
                    WorkspaceRuntimeError::Failed(format!(
                        "failed to lock symbol index storage: {error}"
                    ))
                })?;
            let storage = SymbolIndexStorage::Persistent(lease.directory().join("index.sqlite3"));
            SymbolIndexRuntime::open_with_lease(code_index.index(), storage, lease).map_err(
                |error| {
                    WorkspaceRuntimeError::Failed(format!("failed to open symbol index: {error}"))
                },
            )
        } else {
            SymbolIndexRuntime::open(code_index.index(), SymbolIndexStorage::Memory).map_err(
                |error| {
                    WorkspaceRuntimeError::Failed(format!("failed to open symbol index: {error}"))
                },
            )
        }
    }

    pub(super) fn symbol_index_service(&self) -> Result<Arc<SymbolIndexRuntime>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .symbol_index
            .clone()
            .ok_or_else(|| RpcError::new(-32092, AppServerErrorName::SymbolIndexUnavailable))
    }

    fn open_code_index_semantic(
        &self,
        code_index: &Arc<CodeIndexRuntime>,
    ) -> Result<Option<Arc<CodeIndexSemanticService>>, WorkspaceRuntimeError> {
        open_code_index_semantic_runtime(
            code_index,
            self.code_index_semantic_models.as_ref(),
            self.semantic_model_provider.as_ref(),
            self.config.as_ref(),
            self.workspace_index_storage.as_ref(),
        )
    }

    pub(crate) fn code_index_semantic_service(&self) -> Option<Arc<CodeIndexSemanticService>> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .code_index_semantic
            .clone()
    }

    pub(super) fn code_index_semantic_job(&self) -> Option<Arc<SemanticIndexJobController>> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .code_index_semantic_job
            .clone()
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

    pub(super) fn file_system_service_for(
        &self,
        workspace_folder_id: Option<&str>,
    ) -> Result<Arc<dyn WorkspaceFileSystem>, RpcError> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(workspace_folder_id) = workspace_folder_id {
            return runtime
                .folder_file_systems
                .get(workspace_folder_id)
                .cloned()
                .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        runtime
            .file_system
            .clone()
            .ok_or_else(|| RpcError::new(-32040, AppServerErrorName::FileSystemUnavailable))
    }

    pub(super) fn file_system_service_for_session_directory(
        &self,
        selector: &zeta_app_server_protocol::protocol::workspace::WorkspaceSessionDirectorySelector,
        capability: WorkspaceCapability,
    ) -> Result<Arc<dyn WorkspaceFileSystem>, RpcError> {
        let workspace = self.session_additional_directory_workspace(
            &selector.session_id,
            &selector.root,
            capability,
        )?;
        Ok(Arc::new(LocalFileSystem::new(workspace.root().clone())))
    }

    pub(super) fn language_workspace_root_for(
        &self,
        workspace_folder_id: Option<&str>,
        session_directory: Option<
            &zeta_app_server_protocol::protocol::workspace::WorkspaceSessionDirectorySelector,
        >,
    ) -> Result<WorkspaceRoot, RpcError> {
        if workspace_folder_id.is_some() && session_directory.is_some() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        if let Some(selector) = session_directory {
            let language = self.session_additional_directory_workspace(
                &selector.session_id,
                &selector.root,
                WorkspaceCapability::UseLanguageServices,
            )?;
            self.session_additional_directory_workspace(
                &selector.session_id,
                &selector.root,
                WorkspaceCapability::ExecuteProcess,
            )?;
            return Ok(language.root().clone());
        }
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let authorization = match workspace_folder_id {
            Some(id) => runtime
                .workspace_folders
                .get(id)
                .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams))?,
            None => runtime.authorization.as_ref().ok_or_else(|| {
                RpcError::new(-32040, AppServerErrorName::LanguageServiceUnavailable)
            })?,
        };
        authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| RpcError::new(-32043, AppServerErrorName::WorkspaceTrustRequired))?;
        Ok(authorization.root().clone())
    }

    pub(super) fn git_runtime_service(&self) -> Result<Arc<GitRuntime>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .git
            .clone()
            .ok_or_else(|| RpcError::new(-32060, AppServerErrorName::GitUnavailable))
    }

    pub(super) fn workspace_search_service_for(
        &self,
        workspace_folder_id: Option<&str>,
    ) -> Result<Arc<WorkspaceSearchService>, RpcError> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(workspace_folder_id) = workspace_folder_id {
            return runtime
                .folder_workspace_search
                .get(workspace_folder_id)
                .cloned()
                .ok_or_else(|| RpcError::new(-32050, AppServerErrorName::SearchUnavailable));
        }
        runtime
            .workspace_search
            .clone()
            .ok_or_else(|| RpcError::new(-32050, AppServerErrorName::SearchUnavailable))
    }

    pub(super) fn workspace_search_service_for_session_directory(
        &self,
        selector: &zeta_app_server_protocol::protocol::workspace::WorkspaceSessionDirectorySelector,
    ) -> Result<Arc<WorkspaceSearchService>, RpcError> {
        let workspace = self.session_additional_directory_workspace(
            &selector.session_id,
            &selector.root,
            WorkspaceCapability::SearchRepositoryContent,
        )?;
        let key = (
            selector.session_id.clone(),
            workspace.root().canonical_path().to_path_buf(),
        );
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(search) = runtime.session_additional_directory_search.get(&key) {
            return Ok(Arc::clone(search));
        }
        let ripgrep = runtime
            .ripgrep
            .clone()
            .ok_or_else(|| RpcError::new(-32050, AppServerErrorName::SearchUnavailable))?;
        let search = Arc::new(
            WorkspaceSearchService::new_authorized(workspace, ripgrep)
                .map_err(|_| RpcError::new(-32043, AppServerErrorName::WorkspaceTrustRequired))?,
        );
        runtime
            .session_additional_directory_search
            .insert(key, Arc::clone(&search));
        Ok(search)
    }

    pub(super) fn terminal_service(
        &self,
    ) -> Result<Arc<crate::terminal_service::TerminalService>, RpcError> {
        self.terminal_service_for(None)
    }

    pub(super) fn session_additional_directory_workspace(
        &self,
        session_id: &SessionId,
        root: &std::path::Path,
        capability: WorkspaceCapability,
    ) -> Result<zeta_workspace::TrustedWorkspace, RpcError> {
        self.sessions
            .read_session(session_id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        runtime
            .session_workspace_access
            .workspace_for(session_id, root, capability)
            .map_err(|_| RpcError::new(-32064, AppServerErrorName::TerminalOperationFailed))?
            .ok_or_else(|| RpcError::new(-32064, AppServerErrorName::TerminalOperationFailed))
    }

    fn terminate_revoked_terminal_sessions(&self) {
        for terminals in self.configured_terminal_services() {
            terminals.terminate_revoked_workspaces();
        }
    }

    pub(super) fn terminal_service_for(
        &self,
        workspace_folder_id: Option<&str>,
    ) -> Result<Arc<crate::terminal_service::TerminalService>, RpcError> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(workspace_folder_id) = workspace_folder_id {
            return runtime
                .folder_terminals
                .get(workspace_folder_id)
                .cloned()
                .ok_or_else(|| RpcError::new(-32060, AppServerErrorName::TerminalUnavailable));
        }
        runtime
            .terminals
            .clone()
            .ok_or_else(|| RpcError::new(-32060, AppServerErrorName::TerminalUnavailable))
    }

    pub(super) fn configured_terminal_services(
        &self,
    ) -> Vec<Arc<crate::terminal_service::TerminalService>> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut services = Vec::new();
        if let Some(primary) = &runtime.terminals {
            services.push(Arc::clone(primary));
        }
        for service in runtime.folder_terminals.values() {
            if !services
                .iter()
                .any(|existing| Arc::ptr_eq(existing, service))
            {
                services.push(Arc::clone(service));
            }
        }
        services
    }

    pub(super) fn debug_adapter_service_for(
        &self,
        workspace_folder_id: Option<&str>,
    ) -> Result<Arc<crate::debug_service::DebugAdapterService>, RpcError> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(workspace_folder_id) = workspace_folder_id {
            return runtime
                .folder_debug_adapters
                .get(workspace_folder_id)
                .cloned()
                .ok_or_else(|| RpcError::new(-32070, AppServerErrorName::DebugAdapterUnavailable));
        }
        runtime
            .debug_adapters
            .clone()
            .ok_or_else(|| RpcError::new(-32070, AppServerErrorName::DebugAdapterUnavailable))
    }

    pub(super) fn configured_debug_adapter_services(
        &self,
    ) -> Vec<Arc<crate::debug_service::DebugAdapterService>> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut services = Vec::new();
        if let Some(primary) = &runtime.debug_adapters {
            services.push(Arc::clone(primary));
        }
        for service in runtime.folder_debug_adapters.values() {
            if !services
                .iter()
                .any(|existing| Arc::ptr_eq(existing, service))
            {
                services.push(Arc::clone(service));
            }
        }
        services
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

pub(super) fn read_additional_workspace_config(
    root: &WorkspaceRoot,
) -> Result<zeta_config::WorkspaceConfigDocument, WorkspaceRuntimeError> {
    let digest = Sha256::digest(root.canonical_path().to_string_lossy().as_bytes());
    let workspace_id = WorkspaceId::new(format!(
        "additional-{}",
        digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
    .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
    WorkspaceConfigStore::open(
        root.canonical_path().join(".zeta/config.toml"),
        WorkspaceConfigScope::new(workspace_id),
    )
    .read_document()
    .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))
}

fn session_additional_directory_snapshots(
    access: &SessionWorkspaceAccess,
    session_id: &SessionId,
) -> SessionAdditionalDirectorySnapshotSet {
    let directories = access
        .list(session_id)
        .iter()
        .map(|directory| SessionAdditionalDirectorySnapshot {
            root: directory.root().canonical_path().to_path_buf(),
            decision: directory.decision(),
            permissions: directory.permissions().clone(),
        })
        .collect();
    SessionAdditionalDirectorySnapshotSet {
        revision: access.revision(session_id),
        directories,
    }
}

fn append_multi_agent_tools(
    local: crate::local_tools::LocalToolComposition,
    coordinator: &Arc<MultiAgentCoordinator>,
    sessions: &Arc<SessionCoordinator>,
    turn_backend: &Arc<dyn zeta_core::TurnExecutionBackend>,
    customizations: Option<&Arc<WorkspaceCustomizations>>,
) -> crate::local_tools::LocalToolComposition {
    let action_policy_revision = local.action_policy_revision().clone();
    let local = append_local_tool(
        local,
        Arc::new(
            UpdatePlanToolService::new(Arc::clone(sessions))
                .with_action_policy_revision(action_policy_revision.clone()),
        ),
    );
    let local = append_local_tool(
        local,
        Arc::new(
            GoalToolService::new(Arc::clone(sessions))
                .with_action_policy_revision(action_policy_revision.clone()),
        ),
    );
    let mut multi_agent = MultiAgentToolService::new(
        Arc::clone(coordinator),
        Arc::clone(sessions),
        Arc::clone(turn_backend),
    )
    .with_action_policy_revision(action_policy_revision);
    if let Some(customizations) = customizations {
        multi_agent = multi_agent.with_workspace_customizations(Arc::clone(customizations));
    }
    append_local_tool(local, Arc::new(multi_agent))
}

fn open_code_index_semantic_runtime(
    code_index: &Arc<CodeIndexRuntime>,
    fixed_models: Option<&CodeIndexSemanticModels>,
    provider: Option<&Arc<dyn SemanticModelProvider>>,
    config: Option<&Arc<ConfigStore>>,
    index_storage: Option<&Arc<WorkspaceIndexStorage>>,
) -> Result<Option<Arc<CodeIndexSemanticService>>, WorkspaceRuntimeError> {
    let configured_models;
    let models = match fixed_models {
        Some(models) => models,
        None => {
            let (Some(provider), Some(config)) = (provider, config) else {
                return Ok(None);
            };
            let Some(resolved) =
                resolve_configured_code_index_semantic_models(code_index, provider, config)
            else {
                return Ok(None);
            };
            configured_models = resolved;
            &configured_models
        }
    };
    let trust_id = code_index.root().trust_id();
    let lease = index_storage
        .map(|storage| storage.acquire(&trust_id, WorkspaceIndexKind::Semantic))
        .transpose()
        .map_err(|error| {
            WorkspaceRuntimeError::Failed(format!("failed to lock semantic index storage: {error}"))
        })?;
    let storage = lease
        .as_ref()
        .map_or(CodeIndexSemanticStorage::Memory, |lease| {
            CodeIndexSemanticStorage::Persistent(lease.directory().join("index.sqlite3"))
        });
    let store: Arc<dyn CodeIndexVectorStore> =
        Arc::new(SqliteCodeIndexVectorStore::open(&storage).map_err(|error| {
            WorkspaceRuntimeError::Failed(format!("failed to open semantic code index: {error}"))
        })?);
    let service = CodeIndexSemanticService::new(
        code_index.index(),
        models.model_id.clone(),
        Arc::clone(&models.embedding),
        store,
    );
    let mut service = match &models.rerank {
        Some(rerank) => service.with_rerank(Arc::clone(rerank)),
        None => service,
    };
    if let Some(lease) = lease {
        service = service.with_storage_lease(Arc::new(lease));
    }
    Ok(Some(Arc::new(
        service.with_metrics(Arc::new(AppServerSemanticIndexMetrics)),
    )))
}

fn resolve_configured_code_index_semantic_models(
    code_index: &Arc<CodeIndexRuntime>,
    provider: &Arc<dyn SemanticModelProvider>,
    config: &Arc<ConfigStore>,
) -> Option<CodeIndexSemanticModels> {
    let snapshot = config.read_snapshot().ok()?;
    let models = snapshot
        .values
        .semantic_code_index
        .authorized_remote_models(&code_index.root().trust_id(), &snapshot.values.providers)?
        .clone();
    let invokers =
        match resolve_semantic_model_invokers(provider, &models, &snapshot.values.providers) {
            Ok(invokers) => invokers,
            Err(error) => {
                log::warn!("configured semantic code-index models are unavailable: {error}");
                return None;
            }
        };
    let identity =
        serde_json::to_vec(&(models.embedding_model.clone(), invokers.embedding_config)).ok()?;
    let model_id =
        CodeIndexEmbeddingModelId::new(format!("semantic:sha256:{:x}", Sha256::digest(identity)))
            .ok()?;
    let consent = SemanticInvocationConsent {
        config: Arc::clone(config),
        workspace: code_index.root().trust_id(),
        models: models.clone(),
    };
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(ConsentBoundEmbeddingInvoker {
        inner: invokers.embedding,
        consent: consent.clone(),
    });
    let mut resolved = CodeIndexSemanticModels::new(model_id, embedding);
    if let Some(rerank) = invokers.rerank {
        resolved = resolved.with_rerank(Arc::new(ConsentBoundRerankInvoker {
            inner: rerank,
            consent,
        }));
    }
    Some(resolved)
}

struct ResolvedSemanticModelInvokers {
    embedding_config: ModelProviderConfig,
    embedding: Arc<dyn EmbeddingInvoker>,
    rerank: Option<Arc<dyn RerankInvoker>>,
}

fn resolve_semantic_model_invokers(
    provider: &Arc<dyn SemanticModelProvider>,
    models: &SemanticCodeIndexModelSelection,
    providers: &BTreeMap<ProviderId, ModelProviderConfig>,
) -> Result<ResolvedSemanticModelInvokers, ModelProviderError> {
    let embedding_config = providers
        .get(&models.embedding_model.provider)
        .cloned()
        .ok_or_else(|| {
            ModelProviderError::Unavailable(format!(
                "semantic embedding provider '{}' is not configured",
                models.embedding_model.provider
            ))
        })?;
    let embedding = provider.embedding_runtime(EmbeddingRuntimeRequest::new(
        models.embedding_model.clone(),
        embedding_config.clone(),
    ))?;
    let rerank = models
        .rerank_model
        .as_ref()
        .map(|model| {
            let config = providers.get(&model.provider).cloned().ok_or_else(|| {
                ModelProviderError::Unavailable(format!(
                    "semantic rerank provider '{}' is not configured",
                    model.provider
                ))
            })?;
            provider.rerank_runtime(RerankRuntimeRequest::new(model.clone(), config))
        })
        .transpose()?;
    Ok(ResolvedSemanticModelInvokers {
        embedding_config,
        embedding,
        rerank,
    })
}

#[derive(Clone)]
struct SemanticInvocationConsent {
    config: Arc<ConfigStore>,
    workspace: zeta_workspace::WorkspaceTrustId,
    models: zeta_config::SemanticCodeIndexModelSelection,
}

impl SemanticInvocationConsent {
    fn ensure_current(&self) -> Result<(), ModelProviderError> {
        let snapshot = self
            .config
            .read_snapshot()
            .map_err(|error| ModelProviderError::Unavailable(error.to_string()))?;
        let authorized = snapshot
            .values
            .semantic_code_index
            .authorized_remote_models(&self.workspace, &snapshot.values.providers);
        if authorized == Some(&self.models) {
            Ok(())
        } else {
            Err(ModelProviderError::Unavailable(
                "semantic code-index source egress is no longer authorized".into(),
            ))
        }
    }
}

struct ConsentBoundEmbeddingInvoker {
    inner: Arc<dyn EmbeddingInvoker>,
    consent: SemanticInvocationConsent,
}

impl EmbeddingInvoker for ConsentBoundEmbeddingInvoker {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        self.consent.ensure_current()?;
        self.inner.embed(request)
    }

    fn embed_with_cancellation(
        &self,
        request: &EmbeddingRequest,
        cancellation: &zeta_async_utils::CancellationToken,
    ) -> Result<EmbeddingResponse, ModelProviderError> {
        self.consent.ensure_current()?;
        self.inner.embed_with_cancellation(request, cancellation)
    }
}

struct ConsentBoundRerankInvoker {
    inner: Arc<dyn RerankInvoker>,
    consent: SemanticInvocationConsent,
}

impl RerankInvoker for ConsentBoundRerankInvoker {
    fn rerank(&self, request: &RerankRequest) -> Result<RerankResponse, ModelProviderError> {
        self.consent.ensure_current()?;
        self.inner.rerank(request)
    }

    fn rerank_with_cancellation(
        &self,
        request: &RerankRequest,
        cancellation: &zeta_async_utils::CancellationToken,
    ) -> Result<RerankResponse, ModelProviderError> {
        self.consent.ensure_current()?;
        self.inner.rerank_with_cancellation(request, cancellation)
    }
}

fn retire_workspace_runtime(
    mut runtime: WorkspaceRuntime,
    retained_search: Option<&Arc<WorkspaceSearchService>>,
    retained_terminals: Option<&Arc<crate::terminal_service::TerminalService>>,
    retained_debug_adapters: Option<&Arc<crate::debug_service::DebugAdapterService>>,
) {
    for (_, search) in std::mem::take(&mut runtime.folder_workspace_search) {
        if !retained_search.is_some_and(|retained| Arc::ptr_eq(retained, &search)) {
            search.cancel_all();
        }
    }
    for (_, terminals) in std::mem::take(&mut runtime.folder_terminals) {
        if !retained_terminals.is_some_and(|retained| Arc::ptr_eq(retained, &terminals)) {
            terminals.terminate_all();
        }
    }
    for (_, debug_adapters) in std::mem::take(&mut runtime.folder_debug_adapters) {
        if !retained_debug_adapters.is_some_and(|retained| Arc::ptr_eq(retained, &debug_adapters)) {
            debug_adapters.terminate_all();
        }
    }
    for (_, authorization) in std::mem::take(&mut runtime.workspace_folders) {
        authorization.revoke();
    }
    if let Some(authorization) = runtime.authorization.take() {
        authorization.revoke();
    }
    if let Some(terminals) = runtime.terminals.take()
        && !retained_terminals.is_some_and(|retained| Arc::ptr_eq(retained, &terminals))
    {
        terminals.terminate_all();
    }
    if let Some(debug_adapters) = runtime.debug_adapters.take()
        && !retained_debug_adapters.is_some_and(|retained| Arc::ptr_eq(retained, &debug_adapters))
    {
        debug_adapters.terminate_all();
    }
    if let Some(search) = runtime.workspace_search.take()
        && !retained_search.is_some_and(|retained| Arc::ptr_eq(retained, &search))
    {
        search.cancel_all();
    }
}

#[cfg(test)]
#[path = "workspace_runtime_tests.rs"]
mod tests;
