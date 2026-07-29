use crate::resource_store::{ResourceError, ResourceStore};
use crate::server::skills_runtime::{SkillConfigSnapshotProvider, SkillRuntime, SkillWatcher};
use crate::slash_commands::SlashCommandCatalog;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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
    TurnExecutionLimits, TurnExecutor,
};
use zeta_file_system::WorkspaceFileSystem;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_typst::TypstCompiler;

mod config_operations;
mod fs_operations;
mod operations;
mod search_operations;
mod skill_operations;
pub(crate) mod skills_runtime;
mod update_broker;

use update_broker::{NotificationQueue, UpdateBroker};

pub struct AppServer {
    pub(super) sessions: Arc<SessionCoordinator>,
    model: Arc<dyn ModelService>,
    pub(super) turn_executor: TurnExecutor,
    next_connection_id: AtomicU64,
    pub(super) resources: Mutex<ResourceStore>,
    pub(super) config: Option<Arc<ConfigStore>>,
    pub(super) file_system: Option<Arc<dyn WorkspaceFileSystem>>,
    pub(super) workspace_search: Option<crate::workspace_search::WorkspaceSearchService>,
    pub(super) typst: TypstCompiler,
    pub(super) slash_commands: SlashCommandCatalog,
    pub(super) skills: Option<Arc<SkillRuntime>>,
    _skill_watcher: Option<SkillWatcher>,
    updates: Arc<UpdateBroker>,
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionState {
    pub(super) connection_id: u64,
    initialized: bool,
    request_ids: BTreeSet<u64>,
    outbound_notifications: NotificationQueue,
}

impl AppServer {
    pub fn new(sessions: Arc<SessionCoordinator>, model: Arc<dyn ModelService>) -> Self {
        let updates = Arc::new(UpdateBroker::default());
        let turn_executor = TurnExecutor::without_tools(sessions.threads().clone(), model.clone())
            .with_thread_updates(Arc::new(AppServerThreadUpdates {
                updates: updates.clone(),
            }));
        Self {
            sessions,
            model,
            turn_executor,
            next_connection_id: AtomicU64::new(1),
            resources: Mutex::new(ResourceStore::default()),
            config: None,
            file_system: None,
            workspace_search: None,
            typst: TypstCompiler::new(),
            slash_commands: SlashCommandCatalog::default(),
            skills: None,
            _skill_watcher: None,
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

    pub fn with_config_store(mut self, config: Arc<ConfigStore>) -> Self {
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
        self.file_system = Some(file_system);
        self
    }

    /// Enables connection-owned workspace content search using one frozen ripgrep executable.
    pub fn with_workspace_search(
        mut self,
        workspace: zeta_sandboxing::WorkspaceRoot,
        ripgrep: zeta_shell_command::RipgrepExecutable,
    ) -> Self {
        self.workspace_search = Some(crate::workspace_search::WorkspaceSearchService::new(
            workspace, ripgrep,
        ));
        self
    }

    /// Installs the tool registry and policy used by every Turn executed by this server.
    pub fn with_tool_service(
        mut self,
        tools: Arc<dyn ToolService>,
        policy: Arc<dyn PolicyService>,
    ) -> Self {
        self.turn_executor = TurnExecutor::new(
            self.sessions.threads().clone(),
            self.model.clone(),
            tools,
            policy,
            TurnExecutionLimits::default(),
        )
        .with_thread_updates(Arc::new(AppServerThreadUpdates {
            updates: self.updates.clone(),
        }));
        self
    }

    pub fn sessions(&self) -> &Arc<SessionCoordinator> {
        &self.sessions
    }

    pub fn drain_notifications(&self, connection: &mut ConnectionState) -> Vec<String> {
        std::mem::take(
            &mut *connection
                .outbound_notifications
                .lock()
                .expect("notification queue lock poisoned"),
        )
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
        while let Some(line) = transport.read_message()? {
            let response = self.handle_json(&mut connection, &line);
            transport.write_message(&response)?;
            for notification in self.drain_notifications(&mut connection) {
                transport.write_message(&notification)?;
            }
        }
        Ok(())
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
            Some(ClientMethod::SessionCreate) => self.session_create(connection, &request.params),
            Some(ClientMethod::SessionRead) => self.session_read(&request.params),
            Some(ClientMethod::SessionList) => self.session_list(),
            Some(ClientMethod::SessionSubscribe) => {
                self.session_subscribe(connection, &request.params)
            }
            Some(ClientMethod::SessionUnsubscribe) => {
                self.session_unsubscribe(connection, &request.params)
            }
            Some(ClientMethod::SessionThreadCreate) => {
                self.session_thread_create(connection, &request.params)
            }
            Some(ClientMethod::SessionThreadFork) => {
                self.session_thread_fork(connection, &request.params)
            }
            Some(ClientMethod::SessionThreadArchive) => {
                self.session_thread_archive(connection, &request.params)
            }
            Some(ClientMethod::SessionComplete) => {
                self.session_complete(connection, &request.params)
            }
            Some(ClientMethod::SessionArchive) => self.session_archive(connection, &request.params),
            Some(ClientMethod::ThreadRead) => self.thread_read(&request.params),
            Some(ClientMethod::ThreadSubscribe) => {
                self.thread_subscribe(connection, &request.params)
            }
            Some(ClientMethod::ThreadUnsubscribe) => {
                self.thread_unsubscribe(connection, &request.params)
            }
            Some(ClientMethod::TurnStart) => self.turn_start(connection, &request.params),
            Some(ClientMethod::TurnInterrupt) => self.turn_interrupt(connection, &request.params),
            Some(ClientMethod::TurnInteractionResolve) => {
                self.turn_interaction_resolve(connection, &request.params)
            }
            Some(ClientMethod::TypstCompile) => self.typst_compile(connection, &request.params),
            Some(ClientMethod::ConfigRead) => self.config_read(),
            Some(ClientMethod::ConfigUpdate) => self.config_update(&request.params),
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
            Some(ClientMethod::SkillList) => self.skill_list(&request.params),
            Some(ClientMethod::SkillSetEnablement) => self.skill_set_enablement(&request.params),
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
            Some(ClientMethod::WorkspaceSearchStart) => {
                self.workspace_search_start(connection, &request.params)
            }
            Some(ClientMethod::WorkspaceSearchRead) => {
                self.workspace_search_read(connection, &request.params)
            }
            Some(ClientMethod::WorkspaceSearchCancel) => {
                self.workspace_search_cancel(connection, &request.params)
            }
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
