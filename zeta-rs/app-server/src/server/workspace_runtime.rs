use super::CodeIndexSemanticModels;
use super::code_index_runtime::CodeIndexRuntime;
use super::fs_watcher::FileSystemWatcher;
use super::git_runtime::{GitRuntime, GitWatcher};
use super::goal_tool::GoalToolService;
use super::multi_agent_tools::MultiAgentToolService;
use super::semantic_index_job::AppServerSemanticIndexMetrics;
use super::semantic_index_job::SemanticIndexJobController;
use super::symbol_index_runtime::SymbolIndexRuntime;
use super::update_plan_tool::UpdatePlanToolService;
use super::workspace_customizations::WorkspaceCustomizations;
use super::{AppServer, AppServerThreadUpdates, RpcError};
use crate::code_retrieval_context::CodeRetrievalContextSource;
use crate::code_retrieval_tool::CodeRetrievalTool;
use crate::dynamic_tools::DynamicToolCompositionError;
use crate::dynamic_tools::compose_dynamic_tools;
use crate::local_tools::LocalExecPolicyConfig;
use crate::local_tools::append_local_tool;
use crate::local_tools::compose_local_tools_with_config;
use crate::review::ApprovalModeActionPolicyService;
use crate::session_workspace_roots::SessionWorkspaceRoots;
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
use std::sync::{Arc, Mutex, RwLock};
use zeta_add_dir::AdditionalDirectorySource;
use zeta_add_dir::DirectoryAccessScope;
use zeta_add_dir::DirectoryScopeMutation;
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
use zeta_core::{
    InterruptTurnRequest, MultiAgentCoordinator, SequenceExpectation, SessionCoordinator,
    ThreadController, TurnExecutor,
};
use zeta_file_system::{LocalFileSystem, WorkspaceFileSystem};
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
use zeta_protocol::ProviderId;
use zeta_protocol::{CommandId, SessionId, TurnStatus};
use zeta_search::SearchService;
use zeta_shell_command::RipgrepExecutable;
use zeta_symbol_index::SymbolIndexStorage;
use zeta_tools::ToolRegistryGeneration;
use zeta_workspace::{
    WorkspaceAuthorization, WorkspaceCapability, WorkspaceRoot, WorkspaceTrustDecision,
};

pub(super) struct WorkspaceRuntime {
    pub(super) authorization: Option<WorkspaceAuthorization>,
    pub(super) file_system: Option<Arc<dyn WorkspaceFileSystem>>,
    pub(super) workspace_folders: BTreeMap<String, WorkspaceAuthorization>,
    pub(super) folder_file_systems: BTreeMap<String, Arc<dyn WorkspaceFileSystem>>,
    pub(super) _file_system_watcher: Option<FileSystemWatcher>,
    pub(super) _folder_file_system_watchers: Vec<FileSystemWatcher>,
    pub(super) _git_watcher: Option<GitWatcher>,
    pub(super) git: Option<Arc<GitRuntime>>,
    pub(super) workspace_search: Option<Arc<SearchService>>,
    pub(super) folder_workspace_search: BTreeMap<String, Arc<SearchService>>,
    pub(super) ripgrep: Option<RipgrepExecutable>,
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
    additional_directories: BTreeMap<SessionId, SessionAdditionalDirectories>,
    session_workspace_roots: Arc<SessionWorkspaceRoots>,
    pub(super) turn_executor: TurnExecutor,
}

struct SessionAdditionalDirectories {
    scope: DirectoryAccessScope,
    authorizations: BTreeMap<PathBuf, WorkspaceAuthorization>,
}

pub(super) struct SessionAdditionalDirectorySnapshot {
    pub(super) root: PathBuf,
    pub(super) decision: WorkspaceTrustDecision,
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
            _git_watcher: None,
            git: None,
            workspace_search: None,
            folder_workspace_search: BTreeMap::new(),
            ripgrep: None,
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
            additional_directories: BTreeMap::new(),
            session_workspace_roots: Arc::new(SessionWorkspaceRoots::default()),
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
    exec_policy_config: Arc<RwLock<LocalExecPolicyConfig>>,
    code_index_semantic_storage_root: Option<PathBuf>,
    code_index_semantic_models: Option<CodeIndexSemanticModels>,
    semantic_model_provider: Option<Arc<dyn SemanticModelProvider>>,
    extension_hosts: Option<super::extension_host_runtime::ExtensionHostRuntime>,
}

impl WorkspaceRuntimeControl {
    pub(crate) fn reconcile_exec_policy_config(
        &self,
        config: &zeta_config::ResolvedConfig,
    ) -> Result<(), WorkspaceRuntimeError> {
        let _authority = self.authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let policy_config = LocalExecPolicyConfig::from_resolved(config);
        let (
            authorization,
            code_index,
            symbol_index,
            semantic,
            cloud,
            customizations,
            session_workspace_roots,
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
                Arc::clone(&runtime.session_workspace_roots),
            )
        };
        let Some(authorization) = authorization else {
            *self
                .exec_policy_config
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = policy_config;
            return Ok(());
        };
        if authorization.decision() == WorkspaceTrustDecision::Restricted {
            *self
                .exec_policy_config
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = policy_config;
            return Ok(());
        }
        let execution = authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| WorkspaceRuntimeError::TrustRequired)?;
        let mut local = compose_local_tools_with_config(
            execution.clone(),
            &policy_config,
            session_workspace_roots,
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
        let local_port = local
            .tool_port()
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        self.tools.replace_local(Some(local_port))?;
        *self
            .exec_policy_config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = policy_config;
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
            self.code_index_semantic_storage_root.as_ref(),
        );
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
        let policy_config = self
            .exec_policy_config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let session_workspace_roots = self
            .runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .session_workspace_roots
            .clone();
        let local = compose_local_tools_with_config(
            execution.clone(),
            &policy_config,
            session_workspace_roots,
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

    pub(crate) fn with_local_exec_policy_config(self, config: LocalExecPolicyConfig) -> Self {
        *self
            .local_exec_policy_config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = config;
        self
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
                multi_agent: Arc::clone(&self.multi_agent),
                sessions: Arc::clone(&self.sessions),
                turn_backend: self.turn_backend.clone(),
                updates: Arc::clone(&self.updates),
                hooks: Arc::clone(&host.hooks),
                mcp_status: Arc::clone(&self.mcp_status),
                config: self.config.clone(),
                exec_policy_config: Arc::clone(&self.local_exec_policy_config),
                code_index_semantic_storage_root: self.code_index_semantic_storage_root.clone(),
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
        let folder_file_system_watchers = folders
            .iter()
            .skip(1)
            .map(|(id, authorization)| {
                FileSystemWatcher::start_for_workspace_folder(
                    authorization.root().clone(),
                    Arc::clone(&self.updates),
                    id.clone(),
                )
                .map_err(|error| {
                    WorkspaceRuntimeError::Failed(format!(
                        "failed to initialize workspace folder watcher: {error}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.activate_local_workspace(primary.clone(), host)?;
        let (ripgrep, primary_search, primary_terminals, primary_debug_adapters) = {
            let runtime = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                runtime.ripgrep.clone(),
                runtime.workspace_search.clone(),
                runtime.terminals.clone(),
                runtime.debug_adapters.clone(),
            )
        };
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
                        Arc::new(SearchService::new(
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
    ) -> Result<Vec<SessionAdditionalDirectorySnapshot>, WorkspaceRuntimeError> {
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
        Ok(runtime
            .additional_directories
            .get(session_id)
            .map(additional_directory_snapshots)
            .unwrap_or_default())
    }

    pub(super) fn add_session_additional_directory(
        &self,
        session_id: &SessionId,
        root: PathBuf,
    ) -> Result<
        (
            DirectoryScopeMutation,
            Vec<SessionAdditionalDirectorySnapshot>,
        ),
        WorkspaceRuntimeError,
    > {
        self.sessions
            .read_session(session_id)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let _workspace_authority = self.workspace_authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        self.ensure_workspace_switch_is_idle()?;
        let primary = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()
            .map(|authorization| authorization.root().clone())
            .ok_or(WorkspaceRuntimeError::Unavailable)?;
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
        let canonical = authorization.root().canonical_path().to_path_buf();
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directories = runtime
            .additional_directories
            .entry(session_id.clone())
            .or_insert_with(|| SessionAdditionalDirectories {
                scope: DirectoryAccessScope::new(primary),
                authorizations: BTreeMap::new(),
            });
        let mutation = directories
            .scope
            .add_directory(
                authorization.root().clone(),
                AdditionalDirectorySource::SessionCommand,
            )
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        if mutation == DirectoryScopeMutation::AddedDirectory {
            directories.authorizations.insert(canonical, authorization);
        }
        let access = authorized_additional_roots(directories)?;
        let snapshots = additional_directory_snapshots(directories);
        runtime
            .session_workspace_roots
            .replace_additional(session_id.clone(), access);
        Ok((mutation, snapshots))
    }

    pub(super) fn remove_session_additional_directory(
        &self,
        session_id: &SessionId,
        root: &std::path::Path,
    ) -> Result<
        (
            DirectoryScopeMutation,
            Vec<SessionAdditionalDirectorySnapshot>,
        ),
        WorkspaceRuntimeError,
    > {
        self.sessions
            .read_session(session_id)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let _workspace_authority = self.workspace_authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        self.ensure_workspace_switch_is_idle()?;
        let mut runtime = self
            .workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if runtime.authorization.is_none() {
            return Err(WorkspaceRuntimeError::Unavailable);
        }
        let Some(directories) = runtime.additional_directories.get_mut(session_id) else {
            return Ok((DirectoryScopeMutation::NotPresent, Vec::new()));
        };
        let Some((canonical, authorization)) = directories
            .authorizations
            .iter()
            .find(|(canonical, authorization)| {
                canonical.as_path() == root || authorization.root().requested_path() == root
            })
            .map(|(canonical, authorization)| (canonical.clone(), authorization.clone()))
        else {
            return Ok((
                DirectoryScopeMutation::NotPresent,
                additional_directory_snapshots(directories),
            ));
        };
        let mutation = directories.scope.remove_directory(
            authorization.root(),
            AdditionalDirectorySource::SessionCommand,
        );
        if mutation == DirectoryScopeMutation::RemovedDirectory {
            authorization.revoke();
            directories.authorizations.remove(&canonical);
        }
        let snapshots = additional_directory_snapshots(directories);
        let access = authorized_additional_roots(directories)?;
        let empty = directories.authorizations.is_empty();
        if empty {
            runtime.additional_directories.remove(session_id);
        }
        runtime
            .session_workspace_roots
            .replace_additional(session_id.clone(), access);
        Ok((mutation, snapshots))
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
        if let Some(directories) = runtime.additional_directories.remove(session_id) {
            for authorization in directories.authorizations.values() {
                authorization.revoke();
            }
        }
        runtime
            .session_workspace_roots
            .replace_additional(session_id.clone(), Vec::new());
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
            let policy_config = self
                .local_exec_policy_config
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            let session_workspace_roots = self
                .workspace_runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .session_workspace_roots
                .clone();
            let local =
                compose_local_tools_with_config(execution, &policy_config, session_workspace_roots)
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
        let session_workspace_roots = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .session_workspace_roots
            .clone();
        let customizations =
            WorkspaceCustomizations::discover(&canonical_root, session_workspace_roots)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let file_system_watcher = FileSystemWatcher::start_with_observers(
            workspace,
            Arc::clone(&self.updates),
            Arc::clone(&code_index),
            Arc::clone(&symbol_index),
            None,
            customizations.clone(),
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
        current.session_workspace_roots.clear();
        let next = WorkspaceRuntime {
            authorization: Some(authorization),
            file_system: Some(file_system),
            workspace_folders: BTreeMap::new(),
            folder_file_systems: BTreeMap::new(),
            _file_system_watcher: Some(file_system_watcher),
            _folder_file_system_watchers: Vec::new(),
            _git_watcher: None,
            git: Some(Arc::clone(&git)),
            workspace_search: None,
            folder_workspace_search: BTreeMap::new(),
            ripgrep: None,
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
            additional_directories: BTreeMap::new(),
            session_workspace_roots: Arc::clone(&current.session_workspace_roots),
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
        let code_index_semantic = self.open_code_index_semantic(&code_index);
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
        let session_workspace_roots = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .session_workspace_roots
            .clone();
        let customizations =
            WorkspaceCustomizations::discover(&canonical_root, session_workspace_roots)
                .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
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
            Arc::clone(&symbol_index),
            code_index_semantic_job.clone(),
            customizations.clone(),
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
            Arc::new(SearchService::new(workspace.clone(), local.ripgrep.clone()))
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
        current.session_workspace_roots.clear();
        let next = WorkspaceRuntime {
            authorization: Some(authorization),
            file_system: Some(file_system),
            workspace_folders: BTreeMap::new(),
            folder_file_systems: BTreeMap::new(),
            _file_system_watcher: Some(file_system_watcher),
            _folder_file_system_watchers: Vec::new(),
            _git_watcher: None,
            git: Some(Arc::clone(&git)),
            workspace_search: Some(Arc::clone(&workspace_search)),
            folder_workspace_search: BTreeMap::new(),
            ripgrep: Some(ripgrep),
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
            additional_directories: BTreeMap::new(),
            session_workspace_roots: Arc::clone(&current.session_workspace_roots),
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

    fn open_symbol_index_runtime(
        &self,
        code_index: &Arc<CodeIndexRuntime>,
    ) -> Result<Arc<SymbolIndexRuntime>, WorkspaceRuntimeError> {
        let trust_id = code_index.root().trust_id();
        let digest = trust_id
            .as_str()
            .strip_prefix("sha256:")
            .unwrap_or(trust_id.as_str());
        let storage = self.symbol_index_storage_root.as_ref().map_or(
            SymbolIndexStorage::Memory,
            |storage_root| {
                SymbolIndexStorage::Persistent(storage_root.join(format!("{digest}.sqlite3")))
            },
        );
        match SymbolIndexRuntime::open(code_index.index(), storage) {
            Ok(runtime) => Ok(runtime),
            Err(persistent_error) if self.symbol_index_storage_root.is_some() => {
                log::warn!(
                    "persistent symbol-index cache is unavailable; using memory projection: {persistent_error}"
                );
                SymbolIndexRuntime::open(code_index.index(), SymbolIndexStorage::Memory).map_err(
                    |error| {
                        WorkspaceRuntimeError::Failed(format!(
                            "failed to open symbol index: {error}"
                        ))
                    },
                )
            }
            Err(error) => Err(WorkspaceRuntimeError::Failed(format!(
                "failed to open symbol index: {error}"
            ))),
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
    ) -> Option<Arc<CodeIndexSemanticService>> {
        open_code_index_semantic_runtime(
            code_index,
            self.code_index_semantic_models.as_ref(),
            self.semantic_model_provider.as_ref(),
            self.config.as_ref(),
            self.code_index_semantic_storage_root.as_ref(),
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

    pub(super) fn language_workspace_root_for(
        &self,
        workspace_folder_id: Option<&str>,
    ) -> Result<WorkspaceRoot, RpcError> {
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
    ) -> Result<Arc<SearchService>, RpcError> {
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

    pub(super) fn terminal_service(
        &self,
    ) -> Result<Arc<crate::terminal_service::TerminalService>, RpcError> {
        self.terminal_service_for(None)
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

fn additional_directory_snapshots(
    directories: &SessionAdditionalDirectories,
) -> Vec<SessionAdditionalDirectorySnapshot> {
    directories
        .scope
        .additional_directories()
        .iter()
        .filter_map(|directory| {
            let root = directory.root().canonical_path().to_path_buf();
            directories.authorizations.get(&root).map(|authorization| {
                SessionAdditionalDirectorySnapshot {
                    root,
                    decision: authorization.decision(),
                }
            })
        })
        .collect()
}

fn authorized_additional_roots(
    directories: &SessionAdditionalDirectories,
) -> Result<Vec<zeta_workspace::TrustedWorkspace>, WorkspaceRuntimeError> {
    directories
        .authorizations
        .values()
        .map(|authorization| {
            authorization
                .require(WorkspaceCapability::MutateRepository)
                .map_err(|_| WorkspaceRuntimeError::TrustRequired)
        })
        .collect()
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
    storage_root: Option<&PathBuf>,
) -> Option<Arc<CodeIndexSemanticService>> {
    let configured_models;
    let models = match fixed_models {
        Some(models) => models,
        None => {
            configured_models =
                resolve_configured_code_index_semantic_models(code_index, provider?, config?)?;
            &configured_models
        }
    };
    let trust_id = code_index.root().trust_id();
    let digest = trust_id
        .as_str()
        .strip_prefix("sha256:")
        .unwrap_or(trust_id.as_str());
    let storage = storage_root.map_or(CodeIndexSemanticStorage::Memory, |root| {
        CodeIndexSemanticStorage::Persistent(root.join(format!("{digest}.sqlite3")))
    });
    let store: Arc<dyn CodeIndexVectorStore> = match SqliteCodeIndexVectorStore::open(&storage) {
        Ok(store) => Arc::new(store),
        Err(error) => {
            log::warn!("persistent semantic code-index is unavailable: {error}");
            Arc::new(
                SqliteCodeIndexVectorStore::open(&CodeIndexSemanticStorage::Memory)
                    .expect("in-memory semantic store must open"),
            )
        }
    };
    let service = CodeIndexSemanticService::new(
        code_index.index(),
        models.model_id.clone(),
        Arc::clone(&models.embedding),
        store,
    );
    let service = match &models.rerank {
        Some(rerank) => service.with_rerank(Arc::clone(rerank)),
        None => service,
    };
    Some(Arc::new(
        service.with_metrics(Arc::new(AppServerSemanticIndexMetrics)),
    ))
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
    retained_search: Option<&Arc<SearchService>>,
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
