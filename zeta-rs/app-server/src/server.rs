use crate::SlashCommandCatalog;
use crate::model_catalog::{ModelCatalog, unavailable_model_catalog};
use crate::resource_store::{ResourceError, ResourceStore};
use crate::server::skills_runtime::{SkillConfigSnapshotProvider, SkillRuntime, SkillWatcher};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use zeta_app_server_protocol::protocol::error::{AppServerError, AppServerErrorName};
use zeta_app_server_protocol::protocol::registry::{ClientMethod, client_method};
use zeta_app_server_protocol::rpc::{
    JsonRpcFailure, JsonRpcId, JsonRpcRequest, JsonRpcSuccess, JsonRpcVersion,
};
use zeta_app_server_transport::{DEFAULT_MAX_MESSAGE_BYTES, JsonlTransport};
use zeta_config::ConfigStore;
use zeta_core::{
    CoreError, ModelService, PolicyService, SessionCoordinator, ThreadUpdateSink, ToolService,
    TurnExecutor,
};
use zeta_extensions::ExtensionCatalog;
use zeta_extensions::ExtensionRoot;
use zeta_file_system::WorkspaceFileSystem;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_typst::TypstCompiler;

mod collaboration_operations;
mod collaboration_runtime;
mod config_operations;
mod config_runtime;
mod diff_operations;
mod extension_config_operations;
mod extension_operations;
mod fs_operations;
mod fs_watcher;
mod git_operations;
mod git_runtime;
mod interaction_runtime;
mod notification_queue;
mod operations;
mod search_operations;
mod skill_operations;
pub(crate) mod skills_runtime;
mod syntax_operations;
mod terminal_operations;
mod update_broker;
mod workspace_operations;
mod workspace_runtime;

use notification_queue::NotificationListener;
use notification_queue::NotificationQueue;
use update_broker::UpdateBroker;
use workspace_runtime::{LocalWorkspaceHost, WorkspaceRuntime};
pub(crate) use workspace_runtime::{
    WorkspaceRuntimeControl, WorkspaceSwitchTrustPolicy, WorkspaceToolPorts,
};

pub struct AppServer {
    pub(super) sessions: Arc<SessionCoordinator>,
    model: Arc<dyn ModelService>,
    model_catalog: Arc<dyn ModelCatalog>,
    next_connection_id: AtomicU64,
    pub(super) resources: Mutex<ResourceStore>,
    pub(super) collaboration: Mutex<collaboration_runtime::DocumentCollaborationStore>,
    pub(super) extensions: Mutex<ExtensionCatalog>,
    pub(super) config: Option<Arc<ConfigStore>>,
    pub(super) workspace_authority_gate: Arc<Mutex<()>>,
    workspace_runtime: Arc<RwLock<WorkspaceRuntime>>,
    local_workspace_host: Option<LocalWorkspaceHost>,
    pub(super) typst: TypstCompiler,
    pub(super) slash_commands: SlashCommandCatalog,
    pub(super) skills: Option<Arc<SkillRuntime>>,
    _skill_watcher: Option<SkillWatcher>,
    _config_watcher: Option<config_runtime::ConfigWatcher>,
    _tool_config_watcher: Option<crate::local::ToolConfigWatcher>,
    _interaction_deadline_watcher: interaction_runtime::InteractionDeadlineWatcher,
    updates: Arc<UpdateBroker>,
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionState {
    pub(super) connection_id: u64,
    initialized: bool,
    request_ids: BTreeSet<u64>,
    outbound_notifications: NotificationQueue,
}

/// A wakeable source for outbound notifications owned by one App Server connection.
///
/// Connection hosts wait on this source independently from request dispatch, then drain the
/// pending protocol notifications. Closing the connection wakes any blocked listener.
pub struct ConnectionNotifications {
    listener: NotificationListener,
}

impl ConnectionNotifications {
    /// Blocks until notifications are available or the connection closes.
    pub fn wait(&self) -> bool {
        self.listener.wait()
    }

    /// Drains all currently queued notifications as JSON-RPC messages.
    pub fn drain(&self) -> Vec<String> {
        self.listener
            .drain()
            .into_iter()
            .map(serialize_response)
            .collect()
    }

    /// Closes this notification source and wakes blocked listeners.
    pub fn close(&self) {
        self.listener.close();
    }
}

impl AppServer {
    pub fn new(sessions: Arc<SessionCoordinator>, model: Arc<dyn ModelService>) -> Self {
        let updates = Arc::new(UpdateBroker::default());
        let workspace_authority_gate = Arc::new(Mutex::new(()));
        let interaction_deadline_watcher = interaction_runtime::InteractionDeadlineWatcher::start(
            sessions.threads().clone(),
            updates.clone(),
            workspace_authority_gate.clone(),
        );
        let turn_executor = TurnExecutor::without_tools(sessions.threads().clone(), model.clone())
            .with_thread_updates(Arc::new(AppServerThreadUpdates {
                updates: updates.clone(),
            }));
        Self {
            sessions,
            model,
            model_catalog: unavailable_model_catalog(),
            next_connection_id: AtomicU64::new(1),
            resources: Mutex::new(ResourceStore::default()),
            collaboration: Mutex::new(collaboration_runtime::DocumentCollaborationStore::default()),
            extensions: Mutex::new(ExtensionCatalog::default()),
            config: None,
            workspace_authority_gate,
            workspace_runtime: Arc::new(RwLock::new(WorkspaceRuntime::empty(turn_executor))),
            local_workspace_host: None,
            typst: TypstCompiler::new(),
            slash_commands: SlashCommandCatalog::default(),
            skills: None,
            _skill_watcher: None,
            _config_watcher: None,
            _tool_config_watcher: None,
            _interaction_deadline_watcher: interaction_deadline_watcher,
            updates,
        }
    }

    pub fn connection(&self) -> ConnectionState {
        let connection = ConnectionState {
            connection_id: self.next_connection_id.fetch_add(1, Ordering::Relaxed),
            ..ConnectionState::default()
        };
        self.updates
            .register(connection.connection_id, &connection.outbound_notifications);
        connection
    }

    /// Opens a wakeable outbound-notification source for `connection`.
    pub fn connection_notifications(
        &self,
        connection: &ConnectionState,
    ) -> ConnectionNotifications {
        ConnectionNotifications {
            listener: connection.outbound_notifications.listener(),
        }
    }

    /// Releases connection-scoped subscriptions and runtime resources.
    pub fn close_connection(&self, connection: ConnectionState) {
        self.updates.unregister(connection.connection_id);
        connection.outbound_notifications.close();
        if let Ok(mut resources) = self.resources.lock() {
            resources.release_owner(connection.connection_id);
        }
        if let Some(terminals) = self.configured_terminal_service() {
            terminals.close_owner(connection.connection_id);
        }
    }

    pub fn with_config_store(mut self, config: Arc<ConfigStore>) -> Self {
        self._config_watcher = Some(config_runtime::ConfigWatcher::start(
            &config,
            Arc::clone(&self.updates),
        ));
        self.config = Some(config);
        self
    }

    pub fn with_slash_command_catalog(mut self, slash_commands: SlashCommandCatalog) -> Self {
        self.slash_commands = slash_commands;
        self
    }

    pub(crate) fn with_skill_runtime(
        mut self,
        built_in_source: skills_runtime::BuiltInSkillSource,
        config: Arc<dyn SkillConfigSnapshotProvider>,
    ) -> Result<Self, String> {
        let runtime = SkillRuntime::new(built_in_source, config, Arc::clone(&self.updates))?;
        self._skill_watcher = Some(runtime.start_watching());
        self.skills = Some(runtime);
        Ok(self)
    }

    pub fn with_file_system(mut self, file_system: Arc<dyn WorkspaceFileSystem>) -> Self {
        self.workspace_runtime_mut().file_system = Some(file_system);
        self
    }

    pub fn with_extension_roots(mut self, roots: Vec<ExtensionRoot>) -> Self {
        self.extensions = Mutex::new(ExtensionCatalog::new(roots));
        self
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_file_system_watcher(
        mut self,
        workspace: zeta_workspace::WorkspaceRoot,
    ) -> Result<Self, fs_watcher::FileSystemWatcherError> {
        let watcher = fs_watcher::FileSystemWatcher::start(workspace, Arc::clone(&self.updates))?;
        self.workspace_runtime_mut()._file_system_watcher = Some(watcher);
        Ok(self)
    }

    pub(crate) fn with_model_catalog(mut self, model_catalog: Arc<dyn ModelCatalog>) -> Self {
        self.model_catalog = model_catalog;
        self
    }

    /// Enables workspace-scoped Git queries without exposing arbitrary host paths to clients.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_git_root(
        mut self,
        workspace: zeta_workspace::TrustedWorkspace,
    ) -> Result<Self, git_runtime::GitRuntimeError> {
        let runtime = git_runtime::GitRuntime::new(workspace, Arc::clone(&self.updates))?;
        let state = self.workspace_runtime_mut();
        state._git_watcher = Some(runtime.start_watching());
        state.git = Some(runtime);
        Ok(self)
    }

    /// Enables connection-owned workspace content search using one frozen ripgrep executable.
    pub fn with_workspace_search(
        mut self,
        workspace: zeta_workspace::WorkspaceRoot,
        ripgrep: zeta_shell_command::RipgrepExecutable,
    ) -> Self {
        let search = Arc::new(zeta_search::SearchService::new(workspace, ripgrep));
        self.workspace_runtime_mut().workspace_search = Some(search);
        self
    }

    /// Enables connection-owned interactive terminals rooted at one trusted Workspace.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_terminal_root(
        mut self,
        workspace: zeta_workspace::TrustedWorkspace,
    ) -> Result<Self, crate::terminal_service::TerminalError> {
        let terminals = Arc::new(crate::terminal_service::TerminalService::new(workspace)?);
        self.workspace_runtime_mut().terminals = Some(terminals);
        Ok(self)
    }

    /// Installs the tool registry and policy used by every Turn executed by this server.
    pub fn with_tool_service(
        mut self,
        tools: Arc<dyn ToolService>,
        policy: Arc<dyn PolicyService>,
    ) -> Self {
        let executor = TurnExecutor::new(
            self.sessions.threads().clone(),
            self.model.clone(),
            tools,
            policy,
        )
        .with_thread_updates(Arc::new(AppServerThreadUpdates {
            updates: self.updates.clone(),
        }));
        self.workspace_runtime_mut().turn_executor = executor;
        self
    }

    pub(crate) fn with_tool_config_watcher(
        mut self,
        watcher: crate::local::ToolConfigWatcher,
    ) -> Self {
        self._tool_config_watcher = Some(watcher);
        self
    }

    fn workspace_runtime_mut(&mut self) -> &mut WorkspaceRuntime {
        Arc::get_mut(&mut self.workspace_runtime)
            .expect("Workspace runtime cannot be mutated through a builder after it is shared")
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn sessions(&self) -> &Arc<SessionCoordinator> {
        &self.sessions
    }

    /// Re-enqueues durable running Tool continuations after host services are installed.
    pub fn resume_recovered_tool_continuations(&self) -> Result<usize, CoreError> {
        self.turn_executor_snapshot()
            .resume_recovered_tool_continuations()
    }

    pub fn drain_notifications(&self, connection: &mut ConnectionState) -> Vec<String> {
        connection
            .outbound_notifications
            .drain()
            .into_iter()
            .map(serialize_response)
            .collect()
    }

    pub fn create_resource(
        &self,
        connection: &ConnectionState,
        mime_type: String,
        bytes: Vec<u8>,
    ) -> Result<String, String> {
        self.resources
            .lock()
            .map_err(|_| "resource lock poisoned".to_string())?
            .create(
                connection.connection_id,
                mime_type,
                bytes,
                Duration::from_secs(300),
            )
            .map(|resource| resource.resource_id)
            .map_err(resource_error)
    }

    pub fn handle_json(&self, connection: &mut ConnectionState, raw: &str) -> String {
        let raw_request: Value = match serde_json::from_str(raw) {
            Ok(request) => request,
            Err(_) => {
                return serialize_response(error_response(
                    JsonRpcId::Null(()),
                    -32700,
                    AppServerErrorName::ParseError,
                ));
            }
        };
        let request = match serde_json::from_value::<JsonRpcRequest<Value>>(raw_request) {
            Ok(request)
                if request.jsonrpc == JsonRpcVersion::V2
                    && request.id.as_u64().is_some_and(|request_id| request_id > 0) =>
            {
                request
            }
            _ => {
                return serialize_response(error_response(
                    JsonRpcId::Null(()),
                    -32600,
                    AppServerErrorName::InvalidRequest,
                ));
            }
        };
        let request_id = request.id.as_u64().expect("validated request ID");
        if !connection.request_ids.insert(request_id) {
            return serialize_response(error_response(
                request.id,
                -32600,
                AppServerErrorName::InvalidRequest,
            ));
        }
        let response = match self.dispatch(connection, &request) {
            Ok(result) => serde_json::to_value(JsonRpcSuccess::new(request.id, result))
                .expect("JSON-RPC success response must serialize"),
            Err(error) => error_response(request.id, error.code, error.message),
        };
        serialize_response(response)
    }

    pub fn serve_stdio(&self) -> Result<(), std::io::Error> {
        self.serve_jsonl(std::io::stdin().lock(), std::io::stdout().lock())
    }

    pub fn serve_jsonl<R: BufRead, W: Write>(
        &self,
        reader: R,
        writer: W,
    ) -> Result<(), std::io::Error> {
        let mut transport = JsonlTransport::new(reader, writer, DEFAULT_MAX_MESSAGE_BYTES);
        let mut connection = self.connection();
        let result = (|| {
            while let Some(line) = transport.read_message()? {
                let response = self.handle_json(&mut connection, &line);
                transport.write_message(&response)?;
                for notification in self.drain_notifications(&mut connection) {
                    transport.write_message(&notification)?;
                }
            }
            Ok(())
        })();
        self.close_connection(connection);
        result
    }

    fn dispatch(
        &self,
        connection: &mut ConnectionState,
        request: &JsonRpcRequest<Value>,
    ) -> Result<Value, RpcError> {
        if client_method(&request.method) == Some(ClientMethod::Initialize) {
            return self.initialize(connection, &request.params);
        }
        if !connection.initialized {
            return Err(RpcError::new(-32001, AppServerErrorName::NotInitialized));
        }
        match client_method(&request.method) {
            Some(ClientMethod::Initialize) => unreachable!("initialize handled before gate"),
            Some(ClientMethod::WorkspaceSwitch) => self.workspace_switch(&request.params),
            Some(ClientMethod::DocumentCollaborationOpen) => {
                self.document_collaboration_open(connection, &request.params)
            }
            Some(ClientMethod::DocumentCollaborationSubmit) => {
                self.document_collaboration_submit(&request.params)
            }
            Some(ClientMethod::DocumentCollaborationPresencePublish) => {
                self.document_collaboration_presence_publish(&request.params)
            }
            Some(ClientMethod::DocumentCollaborationPresenceRead) => {
                self.document_collaboration_presence_read(&request.params)
            }
            Some(ClientMethod::SessionCreate) => self.session_create(connection, &request.params),
            Some(ClientMethod::SessionRead) => self.session_read(&request.params),
            Some(ClientMethod::SessionList) => self.session_list(),
            Some(ClientMethod::SessionSubscribe) => {
                self.session_subscribe(connection, &request.params)
            }
            Some(ClientMethod::SessionRequest) => self.session_request(connection, &request.params),
            Some(ClientMethod::SessionUnsubscribe) => {
                self.session_unsubscribe(connection, &request.params)
            }
            Some(ClientMethod::SessionThreadRead) => self.session_thread_read(&request.params),
            Some(ClientMethod::SessionThreadSubscribe) => {
                self.session_thread_subscribe(connection, &request.params)
            }
            Some(ClientMethod::SessionThreadUnsubscribe) => {
                self.session_thread_unsubscribe(connection, &request.params)
            }
            Some(ClientMethod::TypstCompile) => self.typst_compile(connection, &request.params),
            Some(ClientMethod::ConfigRead) => self.config_read(),
            Some(ClientMethod::ModelList) => self.model_list(),
            Some(ClientMethod::ConfigUpdate) => self.config_update(&request.params),
            Some(ClientMethod::LanguageServerConfigure) => {
                self.language_server_configure(&request.params)
            }
            Some(ClientMethod::LanguageServerRemove) => {
                self.language_server_remove(&request.params)
            }
            Some(ClientMethod::ProviderConfigure) => self.provider_configure(&request.params),
            Some(ClientMethod::ProviderRemove) => self.provider_remove(&request.params),
            Some(ClientMethod::McpServerUpsert) => self.mcp_server_upsert(&request.params),
            Some(ClientMethod::McpServerRemove) => self.mcp_server_remove(&request.params),
            Some(ClientMethod::McpServerSetEnablement) => {
                self.mcp_server_set_enablement(&request.params)
            }
            Some(ClientMethod::SkillSourceAdd) => self.skill_source_add(&request.params),
            Some(ClientMethod::SkillSourceRemove) => self.skill_source_remove(&request.params),
            Some(ClientMethod::SkillSourceSetEnablement) => {
                self.skill_source_set_enablement(&request.params)
            }
            Some(ClientMethod::PluginRequestUpsert) => self.plugin_request_upsert(&request.params),
            Some(ClientMethod::PluginRequestRemove) => self.plugin_request_remove(&request.params),
            Some(ClientMethod::PluginRequestSetEnablement) => {
                self.plugin_request_set_enablement(&request.params)
            }
            Some(ClientMethod::HookUpsert) => self.hook_upsert(&request.params),
            Some(ClientMethod::HookRemove) => self.hook_remove(&request.params),
            Some(ClientMethod::HookSetEnablement) => self.hook_set_enablement(&request.params),
            Some(ClientMethod::SkillList) => self.skill_list(&request.params),
            Some(ClientMethod::SkillSetEnablement) => self.skill_set_enablement(&request.params),
            Some(ClientMethod::ExtensionList) => self.extension_list(&request.params),
            Some(ClientMethod::ExtensionResourceOpen) => {
                self.extension_resource_open(connection, &request.params)
            }
            Some(ClientMethod::ResourceMetadata) => {
                self.resource_metadata(connection, &request.params)
            }
            Some(ClientMethod::ResourceRead) => self.resource_read(connection, &request.params),
            Some(ClientMethod::ResourceRelease) => {
                self.resource_release(connection, &request.params)
            }
            Some(ClientMethod::FsGetMetadata) => self.fs_get_metadata(&request.params),
            Some(ClientMethod::FsReadDirectory) => self.fs_read_directory(&request.params),
            Some(ClientMethod::FsReadFile) => self.fs_read_file(&request.params),
            Some(ClientMethod::FsReadBinaryFile) => {
                self.fs_read_binary_file(connection, &request.params)
            }
            Some(ClientMethod::DiffCompute) => self.diff_compute(&request.params),
            Some(ClientMethod::SyntaxAnalyze) => self.syntax_analyze(&request.params),
            Some(ClientMethod::FsWriteFile) => self.fs_write_file(&request.params),
            Some(ClientMethod::GitStatus) => self.git_status(),
            Some(ClientMethod::GitTextDiff) => self.git_text_diff(),
            Some(ClientMethod::GitBranchList) => self.git_branch_list(),
            Some(ClientMethod::GitHistory) => self.git_history(),
            Some(ClientMethod::GitBranchSwitch) => self.git_branch_switch(&request.params),
            Some(ClientMethod::GitStage) => self.git_stage(&request.params),
            Some(ClientMethod::GitUnstage) => self.git_unstage(&request.params),
            Some(ClientMethod::GitDiscardWorktree) => self.git_discard_worktree(&request.params),
            Some(ClientMethod::GitCommit) => self.git_commit(&request.params),
            Some(ClientMethod::GitFetch) => self.git_fetch(),
            Some(ClientMethod::GitPull) => self.git_pull(),
            Some(ClientMethod::GitPush) => self.git_push(),
            Some(ClientMethod::WorkspaceSearchStart) => {
                self.workspace_search_start(connection, &request.params)
            }
            Some(ClientMethod::WorkspaceSearchRead) => {
                self.workspace_search_read(connection, &request.params)
            }
            Some(ClientMethod::WorkspaceSearchCancel) => {
                self.workspace_search_cancel(connection, &request.params)
            }
            Some(ClientMethod::TerminalProfileList) => self.terminal_profile_list(&request.params),
            Some(ClientMethod::TerminalCreate) => self.terminal_create(connection, &request.params),
            Some(ClientMethod::TerminalWrite) => self.terminal_write(connection, &request.params),
            Some(ClientMethod::TerminalResize) => self.terminal_resize(connection, &request.params),
            Some(ClientMethod::TerminalRead) => self.terminal_read(connection, &request.params),
            Some(ClientMethod::TerminalClose) => self.terminal_close(connection, &request.params),
            None => Err(RpcError::new(-32601, AppServerErrorName::MethodNotFound)),
        }
    }
}

struct AppServerThreadUpdates {
    updates: Arc<UpdateBroker>,
}

impl ThreadUpdateSink for AppServerThreadUpdates {
    fn publish(&self, update: ThreadUpdateEnvelope) {
        self.updates.publish_thread_update(update);
    }
}

pub(super) struct RpcError {
    code: i64,
    message: AppServerErrorName,
}

impl RpcError {
    pub(super) fn new(code: i64, message: AppServerErrorName) -> Self {
        Self { code, message }
    }
}

pub(super) fn decode<T: for<'a> Deserialize<'a>>(params: &Value) -> Result<T, RpcError> {
    serde_json::from_value(params.clone())
        .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))
}

pub(super) fn result<T: serde::Serialize>(value: &T) -> Result<Value, RpcError> {
    serde_json::to_value(value)
        .map_err(|_| RpcError::new(-32000, AppServerErrorName::InternalError))
}

pub(super) fn core_error(error: CoreError) -> RpcError {
    match error {
        CoreError::CommandConflict => RpcError::new(-32004, AppServerErrorName::CommandConflict),
        CoreError::NotFound(_) => RpcError::new(-32011, AppServerErrorName::CoreOperationFailed),
        _ => RpcError::new(-32010, AppServerErrorName::CoreOperationFailed),
    }
}

fn serialize_response(value: Value) -> String {
    serde_json::to_string(&value).expect("JSON-RPC response must serialize")
}

fn error_response(id: JsonRpcId, code: i64, message: AppServerErrorName) -> Value {
    serde_json::to_value(JsonRpcFailure::new(
        id,
        AppServerError {
            code,
            message,
            data: (),
        },
    ))
    .expect("JSON-RPC error response must serialize")
}

fn resource_error(error: ResourceError) -> String {
    match error {
        ResourceError::NotFound => "ResourceNotFound",
        ResourceError::NotOwner => "ResourceNotOwner",
        ResourceError::TooLarge => "ResourceTooLarge",
        ResourceError::InvalidChunkSize => "InvalidResourceChunkSize",
        ResourceError::InvalidOffset => "InvalidResourceOffset",
    }
    .into()
}
