use crate::resource_store::{ResourceError, ResourceStore};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use zeta_app_server_protocol::common::{SchemaHash, ServerInfo};
use zeta_app_server_protocol::v1::config::{ConfigReadResult, ConfigUpdateParams, ThemeDto};
use zeta_app_server_protocol::v1::initialize::{
    InitializeParams, InitializeResult, ServerCapabilities,
};
use zeta_app_server_protocol::v1::resources::{
    ResourceMetadataParams, ResourceMetadataResult, ResourceReadParams, ResourceReadResult,
    ResourceReleaseParams,
};
use zeta_app_server_protocol::v1::thread::{
    ThreadDto, ThreadListResult, ThreadReadParams, ThreadReadResult, ThreadResumeParams,
    ThreadStartParams, ThreadStartResult, ThreadUnsubscribeParams, TurnDto, TurnStatusDto,
};
use zeta_app_server_protocol::v1::turn::{TurnInterruptParams, TurnStartParams, TurnStartResult};
use zeta_app_server_protocol::{CURRENT_PROTOCOL_VERSION, schema_hash_v1};
use zeta_app_server_transport::{DEFAULT_MAX_MESSAGE_BYTES, JsonlTransport};
use zeta_config::{ConfigStore, ConfigUpdate, Theme};
use zeta_core::{
    AgentModel, IdempotencyLedger, IdempotencyRecord, InMemoryIdempotencyLedger, ThreadManager,
    ThreadSnapshot, TurnStatus,
};

pub struct AppServer {
    threads: Arc<ThreadManager>,
    model: Arc<dyn AgentModel>,
    idempotency: Arc<dyn IdempotencyLedger>,
    next_connection_id: AtomicU64,
    resources: Mutex<ResourceStore>,
    config: Option<Arc<ConfigStore>>,
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionState {
    connection_id: u64,
    initialized: bool,
    protocol_version: Option<u32>,
    subscriptions: BTreeSet<zeta_protocol::ThreadId>,
    outbound_notifications: Vec<Value>,
}

#[derive(Deserialize)]
struct RequestEnvelope {
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

impl AppServer {
    pub fn new(threads: Arc<ThreadManager>, model: Arc<dyn AgentModel>) -> Self {
        Self::with_idempotency_ledger(
            threads,
            model,
            Arc::new(InMemoryIdempotencyLedger::default()),
        )
    }

    /// Builds an app server with the state-root-scoped durable idempotency adapter.
    pub fn with_idempotency_ledger(
        threads: Arc<ThreadManager>,
        model: Arc<dyn AgentModel>,
        idempotency: Arc<dyn IdempotencyLedger>,
    ) -> Self {
        Self {
            threads,
            model,
            idempotency,
            next_connection_id: AtomicU64::new(1),
            resources: Mutex::new(ResourceStore::default()),
            config: None,
        }
    }

    pub fn connection(&self) -> ConnectionState {
        ConnectionState {
            connection_id: self.next_connection_id.fetch_add(1, Ordering::Relaxed),
            ..ConnectionState::default()
        }
    }

    pub fn with_config_store(mut self, config: Arc<ConfigStore>) -> Self {
        self.config = Some(config);
        self
    }

    pub fn threads(&self) -> &Arc<ThreadManager> {
        &self.threads
    }

    /// Returns notifications that were causally produced by the last request in this connection.
    pub fn drain_notifications(&self, connection: &mut ConnectionState) -> Vec<String> {
        std::mem::take(&mut connection.outbound_notifications)
            .into_iter()
            .map(serialize_response)
            .collect()
    }

    /// Registers a server-produced resource for its owner connection.
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

    /// Processes one JSON Lines JSON-RPC request. Callers keep one `ConnectionState` per transport.
    pub fn handle_json(&self, connection: &mut ConnectionState, raw: &str) -> String {
        let request: RequestEnvelope = match serde_json::from_str::<RequestEnvelope>(raw) {
            Ok(request)
                if request.jsonrpc == "2.0"
                    && request.id.as_u64().is_some_and(|request_id| request_id > 0) =>
            {
                request
            }
            Ok(_) => {
                return serialize_response(error_response(
                    Value::Null,
                    -32600,
                    "InvalidRequest",
                    None,
                ));
            }
            Err(_) => {
                return serialize_response(error_response(Value::Null, -32700, "ParseError", None));
            }
        };
        let response = match self.dispatch(connection, &request) {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": request.id, "result": result }),
            Err(error) => error_response(request.id, error.code, error.message, error.data),
        };
        serialize_response(response)
    }

    /// Serves stdio without emitting non-protocol bytes on stdout.
    pub fn serve_stdio(&self) -> Result<(), std::io::Error> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut transport =
            JsonlTransport::new(stdin.lock(), stdout.lock(), DEFAULT_MAX_MESSAGE_BYTES);
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
        request: &RequestEnvelope,
    ) -> Result<Value, RpcError> {
        if request.method == "initialize" {
            return self.initialize(connection, &request.params);
        }
        if !connection.initialized {
            return Err(RpcError::new(-32001, "NotInitialized"));
        }
        match request.method.as_str() {
            "thread/start" => self.thread_start(connection, &request.params),
            "thread/read" => self.thread_read(&request.params),
            "thread/resume" => self.thread_resume(connection, &request.params),
            "thread/list" => self.thread_list(),
            "thread/unsubscribe" => self.thread_unsubscribe(connection, &request.params),
            "config/read" => self.config_read(),
            "config/update" => self.config_update(&request.params),
            "turn/start" => self.turn_start(connection, &request.params),
            "turn/interrupt" => self.turn_interrupt(connection, &request.params),
            "resource/metadata" => self.resource_metadata(connection, &request.params),
            "resource/read" => self.resource_read(connection, &request.params),
            "resource/release" => self.resource_release(connection, &request.params),
            _ => Err(RpcError::new(-32601, "MethodNotFound")),
        }
    }

    fn initialize(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        if connection.initialized {
            return Err(RpcError::new(-32002, "AlreadyInitialized"));
        }
        let params: InitializeParams = decode(params)?;
        if params.client_info.name.trim().is_empty()
            || params.client_info.version.trim().is_empty()
            || params.protocol_versions.min == 0
            || params.protocol_versions.max < params.protocol_versions.min
        {
            return Err(RpcError::new(-32602, "InvalidParams"));
        }
        let version = params.protocol_versions.max.min(CURRENT_PROTOCOL_VERSION);
        if version < params.protocol_versions.min {
            return Err(RpcError::new(-32003, "ProtocolVersionUnsupported"));
        }
        connection.initialized = true;
        connection.protocol_version = Some(version);
        result(&InitializeResult {
            server_info: ServerInfo {
                name: "zeta-app-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            protocol_version: version,
            schema_hash: SchemaHash(schema_hash_v1()),
            capabilities: ServerCapabilities {
                threads: true,
                turns: true,
                resources: true,
            },
        })
    }

    fn thread_start(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ThreadStartParams = decode(params)?;
        require_idempotency_key(&params.idempotency_key)?;
        let idempotency_key = params.idempotency_key.clone();
        let response = self.idempotent("thread/start", idempotency_key, params, |params| {
            let thread_id = self
                .threads
                .start_thread(params.title)
                .map_err(core_error)?;
            let snapshot = self.threads.read_thread(&thread_id).map_err(core_error)?;
            result(&ThreadStartResult {
                thread_id,
                sequence: snapshot.sequence,
            })
        })?;
        let started: ThreadStartResult = serde_json::from_value(response.clone())
            .map_err(|_| RpcError::new(-32000, "InternalError"))?;
        connection.subscriptions.insert(started.thread_id.clone());
        notify(
            connection,
            "thread/started",
            json!({ "threadId": started.thread_id, "sequence": started.sequence }),
        );
        Ok(response)
    }

    fn thread_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ThreadReadParams = decode(params)?;
        result(&ThreadReadResult {
            thread: thread_dto(
                self.threads
                    .read_thread(&params.thread_id)
                    .map_err(core_error)?,
            ),
        })
    }

    fn thread_resume(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ThreadResumeParams = decode(params)?;
        let snapshot = self
            .threads
            .read_thread(&params.thread_id)
            .map_err(core_error)?;
        connection.subscriptions.insert(params.thread_id);
        result(&ThreadReadResult {
            thread: thread_dto(snapshot),
        })
    }

    fn thread_list(&self) -> Result<Value, RpcError> {
        result(&ThreadListResult {
            threads: self
                .threads
                .list_threads()
                .map_err(core_error)?
                .into_iter()
                .map(thread_dto)
                .collect(),
        })
    }

    fn config_read(&self) -> Result<Value, RpcError> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, "ConfigUnavailable"))?
            .read()
            .map_err(config_error)?;
        result(&ConfigReadResult {
            preferred_model: config.preferred_model,
            theme: config.theme.map(theme_dto),
        })
    }

    fn config_update(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ConfigUpdateParams = decode(params)?;
        require_idempotency_key(&params.idempotency_key)?;
        let idempotency_key = params.idempotency_key.clone();
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, "ConfigUnavailable"))?;
        self.idempotent("config/update", idempotency_key, params, move |params| {
            let config = store
                .update(ConfigUpdate {
                    preferred_model: params.preferred_model,
                    theme: params.theme.map(|theme| theme.map(theme_from_dto)),
                })
                .map_err(config_error)?;
            result(&ConfigReadResult {
                preferred_model: config.preferred_model,
                theme: config.theme.map(theme_dto),
            })
        })
    }

    fn thread_unsubscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ThreadUnsubscribeParams = decode(params)?;
        connection.subscriptions.remove(&params.thread_id);
        Ok(Value::Null)
    }

    fn turn_start(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TurnStartParams = decode(params)?;
        require_idempotency_key(&params.idempotency_key)?;
        let idempotency_key = params.idempotency_key.clone();
        let thread_id = params.thread_id.clone();
        let mut completed = None;
        let response = self.idempotent("turn/start", idempotency_key, params, |params| {
            if params.input.is_empty() {
                return Err(RpcError::new(-32602, "InvalidParams"));
            }
            let prompt = params
                .input
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let turn_id = self
                .threads
                .start_turn(&params.thread_id)
                .map_err(core_error)?;
            let started_sequence = self
                .threads
                .read_thread(&params.thread_id)
                .map_err(core_error)?
                .sequence;
            let output = self.model.respond(&prompt).map_err(core_error)?;
            self.threads
                .complete_turn(&params.thread_id, &turn_id)
                .map_err(core_error)?;
            let completed_sequence = self
                .threads
                .read_thread(&params.thread_id)
                .map_err(core_error)?
                .sequence;
            completed = Some((turn_id.clone(), output, completed_sequence));
            result(&TurnStartResult {
                turn_id,
                sequence: started_sequence,
            })
        })?;
        let started: TurnStartResult = serde_json::from_value(response.clone())
            .map_err(|_| RpcError::new(-32000, "InternalError"))?;
        if let Some((turn_id, output, completed_sequence)) = completed {
            notify_for_thread(
                connection,
                &thread_id,
                "turn/started",
                json!({ "threadId": thread_id, "turnId": started.turn_id, "sequence": started.sequence }),
            );
            notify_for_thread(
                connection,
                &thread_id,
                "item/agentMessage/completed",
                json!({
                    "threadId": thread_id,
                    "turnId": turn_id,
                    "text": output,
                    "sequence": completed_sequence
                }),
            );
            notify_for_thread(
                connection,
                &thread_id,
                "turn/completed",
                json!({
                    "threadId": thread_id,
                    "turnId": turn_id,
                    "sequence": completed_sequence
                }),
            );
        }
        Ok(response)
    }

    fn turn_interrupt(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TurnInterruptParams = decode(params)?;
        self.threads
            .interrupt_turn(&params.thread_id, &params.turn_id)
            .map_err(core_error)?;
        let snapshot = self
            .threads
            .read_thread(&params.thread_id)
            .map_err(core_error)?;
        notify_for_thread(
            connection,
            &params.thread_id,
            "turn/interrupted",
            json!({ "threadId": params.thread_id, "turnId": params.turn_id, "sequence": snapshot.sequence }),
        );
        Ok(Value::Null)
    }

    fn resource_metadata(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ResourceMetadataParams = decode(params)?;
        let metadata = self
            .resources
            .lock()
            .map_err(|_| RpcError::new(-32000, "ServerOverloaded"))?
            .metadata(connection.connection_id, &params.resource_id)
            .map_err(resource_rpc_error)?;
        result(&ResourceMetadataResult {
            resource_id: metadata.resource_id,
            mime_type: metadata.mime_type,
            size: metadata.size,
            sha256: metadata.sha256,
        })
    }

    fn resource_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ResourceReadParams = decode(params)?;
        let resource_id = params.resource_id.clone();
        let chunk = self
            .resources
            .lock()
            .map_err(|_| RpcError::new(-32000, "ServerOverloaded"))?
            .read(
                connection.connection_id,
                &params.resource_id,
                params.offset,
                params.max_bytes,
            )
            .map_err(resource_rpc_error)?;
        result(&ResourceReadResult {
            resource_id,
            offset: chunk.offset,
            data: chunk.data,
            eof: chunk.eof,
        })
    }

    fn resource_release(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ResourceReleaseParams = decode(params)?;
        self.resources
            .lock()
            .map_err(|_| RpcError::new(-32000, "ServerOverloaded"))?
            .release(connection.connection_id, &params.resource_id)
            .map_err(resource_rpc_error)?;
        Ok(Value::Null)
    }

    fn idempotent<T: serde::Serialize>(
        &self,
        method: &str,
        key: String,
        params: T,
        operation: impl FnOnce(T) -> Result<Value, RpcError>,
    ) -> Result<Value, RpcError> {
        let parameters =
            serde_json::to_string(&params).map_err(|_| RpcError::new(-32602, "InvalidParams"))?;
        let entry_key = (method.to_owned(), key);
        if let Some(entry) = self
            .idempotency
            .get(&entry_key.0, &entry_key.1)
            .map_err(core_error)?
        {
            return if entry.parameters == parameters {
                serde_json::from_str(&entry.result)
                    .map_err(|_| RpcError::new(-32000, "InternalError"))
            } else {
                Err(RpcError::new(-32004, "IdempotencyConflict"))
            };
        }
        let response = operation(params)?;
        self.idempotency
            .put(IdempotencyRecord {
                method: entry_key.0,
                key: entry_key.1,
                parameters,
                result: serde_json::to_string(&response)
                    .map_err(|_| RpcError::new(-32000, "InternalError"))?,
            })
            .map_err(core_error)?;
        Ok(response)
    }
}

struct RpcError {
    code: i64,
    message: &'static str,
    data: Option<Value>,
}
impl RpcError {
    fn new(code: i64, message: &'static str) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }
}
fn decode<T: for<'a> Deserialize<'a>>(params: &Value) -> Result<T, RpcError> {
    serde_json::from_value(params.clone()).map_err(|_| RpcError::new(-32602, "InvalidParams"))
}
fn result<T: serde::Serialize>(value: &T) -> Result<Value, RpcError> {
    serde_json::to_value(value).map_err(|_| RpcError::new(-32000, "InternalError"))
}
fn require_idempotency_key(key: &str) -> Result<(), RpcError> {
    if key.trim().is_empty() {
        Err(RpcError::new(-32602, "InvalidParams"))
    } else {
        Ok(())
    }
}
fn core_error(_: zeta_core::CoreError) -> RpcError {
    RpcError::new(-32010, "CoreOperationFailed")
}
fn config_error(_: zeta_config::ConfigError) -> RpcError {
    RpcError::new(-32030, "ConfigUnavailable")
}
fn theme_dto(theme: Theme) -> ThemeDto {
    match theme {
        Theme::Light => ThemeDto::Light,
        Theme::Dark => ThemeDto::Dark,
        Theme::System => ThemeDto::System,
    }
}
fn theme_from_dto(theme: ThemeDto) -> Theme {
    match theme {
        ThemeDto::Light => Theme::Light,
        ThemeDto::Dark => Theme::Dark,
        ThemeDto::System => Theme::System,
    }
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
fn resource_rpc_error(error: ResourceError) -> RpcError {
    RpcError::new(
        -32020,
        match error {
            ResourceError::NotFound => "ResourceNotFound",
            ResourceError::NotOwner => "ResourceNotOwner",
            ResourceError::TooLarge => "ResourceTooLarge",
            ResourceError::InvalidChunkSize => "InvalidResourceChunkSize",
            ResourceError::InvalidOffset => "InvalidResourceOffset",
        },
    )
}
fn serialize_response(value: Value) -> String {
    serde_json::to_string(&value).expect("JSON-RPC response must serialize")
}
fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message, "data": data } })
}
fn notify(connection: &mut ConnectionState, method: &str, params: Value) {
    connection
        .outbound_notifications
        .push(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}
fn notify_for_thread(
    connection: &mut ConnectionState,
    thread_id: &zeta_protocol::ThreadId,
    method: &str,
    params: Value,
) {
    if connection.subscriptions.contains(thread_id) {
        notify(connection, method, params);
    }
}
fn thread_dto(snapshot: ThreadSnapshot) -> ThreadDto {
    ThreadDto {
        thread_id: snapshot.thread_id,
        title: snapshot.title,
        sequence: snapshot.sequence,
        turns: snapshot
            .turns
            .into_iter()
            .map(|(turn_id, status)| TurnDto {
                turn_id: turn_id.to_string(),
                status: turn_status(status),
            })
            .collect(),
    }
}
fn turn_status(status: TurnStatus) -> TurnStatusDto {
    match status {
        TurnStatus::Created => TurnStatusDto::Created,
        TurnStatus::Running => TurnStatusDto::Running,
        TurnStatus::WaitingForApproval => TurnStatusDto::WaitingForApproval,
        TurnStatus::WaitingForUserInput => TurnStatusDto::WaitingForUserInput,
        TurnStatus::WaitingForCapability => TurnStatusDto::WaitingForCapability,
        TurnStatus::Cancelling => TurnStatusDto::Cancelling,
        TurnStatus::Completed => TurnStatusDto::Completed,
        TurnStatus::Failed => TurnStatusDto::Failed,
        TurnStatus::Interrupted => TurnStatusDto::Interrupted,
    }
}
