use super::CodeIndexSemanticModels;
use super::code_index_runtime::CodeIndexRuntime;
use super::fs_watcher::FileSystemWatcher;
use super::git_runtime::{GitRuntime, GitWatcher};
use super::multi_agent_tools::MultiAgentToolService;
use super::semantic_index_job::AppServerSemanticIndexMetrics;
use super::semantic_index_job::SemanticIndexJobController;
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
use zeta_protocol::{CommandId, TurnStatus};
use zeta_search::SearchService;
use zeta_tools::ToolRegistryGeneration;
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
    pub(super) code_index_semantic: Option<Arc<CodeIndexSemanticService>>,
    pub(super) code_index_semantic_job: Option<Arc<SemanticIndexJobController>>,
    pub(super) cloud_code_index: Option<Arc<CloudCodeIndexController>>,
    pub(super) _customizations: Option<Arc<WorkspaceCustomizations>>,
    pub(super) terminals: Option<Arc<crate::terminal_service::TerminalService>>,
    pub(super) debug_adapters: Option<Arc<crate::debug_service::DebugAdapterService>>,
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
            code_index_semantic: None,
            code_index_semantic_job: None,
            cloud_code_index: None,
            _customizations: None,
            terminals: None,
            debug_adapters: None,
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
    multi_agent: Arc<MultiAgentCoordinator>,
    sessions: Arc<SessionCoordinator>,
    updates: Arc<super::update_broker::UpdateBroker>,
    config: Option<Arc<ConfigStore>>,
    exec_policy_config: Arc<RwLock<LocalExecPolicyConfig>>,
    code_index_semantic_storage_root: Option<PathBuf>,
    code_index_semantic_models: Option<CodeIndexSemanticModels>,
    semantic_model_provider: Option<Arc<dyn SemanticModelProvider>>,
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
        let (authorization, code_index, semantic, cloud) = {
            let runtime = self
                .runtime
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                runtime.authorization.clone(),
                runtime.code_index.clone(),
                runtime.code_index_semantic.clone(),
                runtime.cloud_code_index.clone(),
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
        let mut local = compose_local_tools_with_config(execution.clone(), &policy_config)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        if let Some(code_index) = code_index {
            let action_policy_revision = local.action_policy_revision().clone();
            local = append_local_tool(
                local,
                Arc::new(
                    CodeRetrievalTool::new(execution, code_index.index(), semantic, cloud)
                        .with_action_policy_revision(action_policy_revision),
                ),
            );
        }
        local = append_multi_agent_tools(local, &self.multi_agent, &self.sessions, &self.runtime);
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

    pub(crate) fn reconcile_semantic_code_index_runtime(
        &self,
    ) -> Result<(), WorkspaceRuntimeError> {
        let _authority = self.authority_gate.lock().map_err(|_| {
            WorkspaceRuntimeError::Failed("Workspace authority gate poisoned".into())
        })?;
        let (authorization, code_index, cloud, customizations, previous_watcher, previous_job) = {
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
            let Some(customizations) = runtime._customizations.clone() else {
                return Ok(());
            };
            let previous_watcher = runtime._file_system_watcher.take();
            let previous_job = runtime.code_index_semantic_job.take();
            runtime.code_index_semantic = None;
            (
                authorization,
                code_index,
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
        let local = compose_local_tools_with_config(execution.clone(), &policy_config)
            .map_err(|error| WorkspaceRuntimeError::Failed(error.to_string()))?;
        let action_policy_revision = local.action_policy_revision().clone();
        let local = append_local_tool(
            local,
            Arc::new(
                CodeRetrievalTool::new(
                    execution,
                    code_index.index(),
                    semantic.clone(),
                    cloud.clone(),
                )
                .with_action_policy_revision(action_policy_revision),
            ),
        );
        let local =
            append_multi_agent_tools(local, &self.multi_agent, &self.sessions, &self.runtime);
        let watcher = FileSystemWatcher::start_with_observers(
            authorization.root().clone(),
            Arc::clone(&self.updates),
            Arc::clone(&code_index),
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
        authorization.revoke();
        let cloud_code_index = runtime.cloud_code_index.clone();
        let old_file_system_watcher = runtime._file_system_watcher.take();
        let code_index = runtime.code_index.clone();
        let customizations = runtime._customizations.clone();
        let terminals = runtime.terminals.take();
        let debug_adapters = runtime.debug_adapters.take();
        let search = runtime.workspace_search.take();
        let git = runtime.git.take();
        let git_watcher = runtime._git_watcher.take();
        runtime.cloud_code_index = None;
        runtime.code_index_semantic = None;
        runtime.code_index_semantic_job = None;
        runtime.authorization = Some(WorkspaceAuthorization::new(
            root.clone(),
            WorkspaceTrustDecision::Restricted,
        ));
        drop(runtime);

        drop(old_file_system_watcher);
        let (restricted_watcher, watcher_error) = match (code_index, customizations) {
            (Some(code_index), Some(customizations)) => {
                match FileSystemWatcher::start_with_observers(
                    root.clone(),
                    Arc::clone(&self.updates),
                    code_index,
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

        let tool_result = self.tools.replace_local(None);
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
    extension: Option<ToolPort>,
    local: Option<ToolPort>,
    mcp: Option<ToolPort>,
    search: ToolSearchOptions,
    search_status: ToolSearchEmbeddingStatus,
    registry_generation: ToolRegistryGeneration,
}

pub(crate) struct WorkspaceToolPorts {
    state: Mutex<WorkspaceToolPortState>,
    reloadable: Arc<ReloadableToolPorts>,
    semantic_model_provider: Option<Arc<dyn SemanticModelProvider>>,
}

impl WorkspaceToolPorts {
    #[cfg(test)]
    pub(crate) fn definitions(&self) -> Vec<zeta_protocol::ToolDefinition> {
        self.reloadable.tools().definitions()
    }

    fn new(
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
                extension,
                local: None,
                mcp,
                search: search.options,
                search_status: search.status,
                registry_generation,
            }),
            reloadable: ReloadableToolPorts::new(combined),
            semantic_model_provider,
        }))
    }

    fn replace_local(&self, local: Option<ToolPort>) -> Result<(), WorkspaceRuntimeError> {
        self.replace(|state| {
            state.local = local;
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
            extension: state.extension.clone(),
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
        let (search_config, providers) = match &self.config {
            Some(config) => {
                let snapshot = config
                    .read_snapshot()
                    .map_err(|error| WorkspaceRuntimeError::Failed(error.0))?;
                (snapshot.values.tool_search, snapshot.values.providers)
            }
            None => (ToolSearchConfig::default(), Default::default()),
        };
        let tools = WorkspaceToolPorts::new(
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
        let mut executor = TurnExecutor::new(
            self.sessions.threads().clone(),
            Arc::clone(&self.model),
            tools.reloadable.tools(),
            policy,
        )
        .with_thread_updates(Arc::new(AppServerThreadUpdates {
            updates: Arc::clone(&self.updates),
        }));
        executor = executor.with_extensions(Arc::clone(&self.agent_extensions));
        self.workspace_runtime
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .turn_executor = executor;
        self.local_workspace_host = Some(LocalWorkspaceHost { tools, trust });
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
                updates: Arc::clone(&self.updates),
                config: self.config.clone(),
                exec_policy_config: Arc::clone(&self.local_exec_policy_config),
                code_index_semantic_storage_root: self.code_index_semantic_storage_root.clone(),
                code_index_semantic_models: self.code_index_semantic_models.clone(),
                semantic_model_provider: self.semantic_model_provider.clone(),
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
        let policy_config = self
            .local_exec_policy_config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let local = compose_local_tools_with_config(execution, &policy_config)
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
            None,
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
            code_index_semantic: None,
            code_index_semantic_job: None,
            cloud_code_index: None,
            _customizations: Some(Arc::clone(&customizations)),
            terminals: None,
            debug_adapters: None,
            turn_executor: current
                .turn_executor
                .clone()
                .with_instructions_provider(customizations)
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
                    code_index_semantic.clone(),
                    cloud_code_index.clone(),
                )
                .with_action_policy_revision(action_policy_revision),
            ),
        );
        let local = append_multi_agent_tools(
            local,
            &self.multi_agent,
            &self.sessions,
            &self.workspace_runtime,
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
        host.tools.replace_local(Some(local_port))?;
        self.bind_workspace_skills(&canonical_root)?;
        let context_source = Arc::new(CodeRetrievalContextSource::new(
            code_index.index(),
            code_index_semantic.clone(),
            cloud_code_index.clone(),
            self.config.clone(),
            workspace.trust_id(),
        ));
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
            code_index_semantic,
            code_index_semantic_job,
            cloud_code_index,
            _customizations: Some(Arc::clone(&customizations)),
            terminals: Some(Arc::clone(&terminals)),
            debug_adapters: Some(Arc::clone(&debug_adapters)),
            turn_executor: current
                .turn_executor
                .clone()
                .with_instructions_provider(customizations)
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

    pub(super) fn file_system_service(&self) -> Result<Arc<dyn WorkspaceFileSystem>, RpcError> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .file_system
            .clone()
            .ok_or_else(|| RpcError::new(-32040, AppServerErrorName::FileSystemUnavailable))
    }

    pub(super) fn language_workspace_root(&self) -> Result<WorkspaceRoot, RpcError> {
        let runtime = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let authorization = runtime
            .authorization
            .as_ref()
            .ok_or_else(|| RpcError::new(-32040, AppServerErrorName::LanguageServiceUnavailable))?;
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

    pub(super) fn debug_adapter_service(
        &self,
    ) -> Result<Arc<crate::debug_service::DebugAdapterService>, RpcError> {
        self.configured_debug_adapter_service()
            .ok_or_else(|| RpcError::new(-32070, AppServerErrorName::DebugAdapterUnavailable))
    }

    pub(super) fn configured_debug_adapter_service(
        &self,
    ) -> Option<Arc<crate::debug_service::DebugAdapterService>> {
        self.workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .debug_adapters
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

fn append_multi_agent_tools(
    local: crate::local_tools::LocalToolComposition,
    coordinator: &Arc<MultiAgentCoordinator>,
    sessions: &Arc<SessionCoordinator>,
    runtime: &Arc<RwLock<WorkspaceRuntime>>,
) -> crate::local_tools::LocalToolComposition {
    let action_policy_revision = local.action_policy_revision().clone();
    append_local_tool(
        local,
        Arc::new(
            MultiAgentToolService::new(
                Arc::clone(coordinator),
                Arc::clone(sessions),
                Arc::downgrade(runtime),
            )
            .with_action_policy_revision(action_policy_revision),
        ),
    )
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
