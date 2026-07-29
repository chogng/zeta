use crate::agent::{AgentCallError, AgentOutcome, AgentService};
mod events;
use crate::protocol::{
    CallToolParams, CancelledParams, IncomingMessage, InitializeParams, JsonRpcId, TOOL_REPLY,
    TOOL_START, decode_reply, decode_start, initialize_result, tools_result,
};
use events::McpAgentEvents;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

const MAX_EARLY_CANCELLATIONS: usize = 1024;
const MAX_TOOL_RESULT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpServerError(String);

impl McpServerError {
    pub(crate) fn app_server(error: impl fmt::Display) -> Self {
        Self(format!("could not open embedded App Server: {error}"))
    }

    pub(crate) fn configuration(error: impl fmt::Display) -> Self {
        Self(format!("invalid MCP server configuration: {error}"))
    }

    pub(crate) fn receipt(error: impl fmt::Display) -> Self {
        Self(format!("could not open MCP receipt store: {error}"))
    }

    pub(crate) fn http(error: impl fmt::Display) -> Self {
        Self(format!("MCP HTTP transport failed: {error}"))
    }

    fn io(error: impl fmt::Display) -> Self {
        Self(format!("stdio transport failed: {error}"))
    }
}

impl fmt::Display for McpServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for McpServerError {}

#[derive(Clone)]
pub(crate) struct McpServer {
    inner: Arc<ServerInner>,
}

struct ServerInner {
    agent: Arc<dyn AgentService>,
    initialized: AtomicBool,
    shutting_down: AtomicBool,
    active_requests: Mutex<HashMap<JsonRpcId, Arc<AtomicBool>>>,
    early_cancellations: Mutex<HashSet<JsonRpcId>>,
    client_features: Mutex<ClientFeatures>,
    pending_server_requests: Mutex<HashMap<JsonRpcId, mpsc::Sender<Result<Value, String>>>>,
    next_server_request: AtomicU64,
}

#[derive(Default)]
struct ClientFeatures {
    elicitation_form: bool,
}

impl McpServer {
    pub(crate) fn new(agent: Arc<dyn AgentService>) -> Self {
        Self {
            inner: Arc::new(ServerInner {
                agent,
                initialized: AtomicBool::new(false),
                shutting_down: AtomicBool::new(false),
                active_requests: Mutex::new(HashMap::new()),
                early_cancellations: Mutex::new(HashSet::new()),
                client_features: Mutex::new(ClientFeatures::default()),
                pending_server_requests: Mutex::new(HashMap::new()),
                next_server_request: AtomicU64::new(1),
            }),
        }
    }

    #[cfg(test)]
    fn handle_line(&self, line: &str) -> Option<String> {
        let (outgoing, _) = mpsc::channel();
        self.handle_line_with_outgoing(line, outgoing)
    }

    pub(crate) fn handle_line_with_outgoing(
        &self,
        line: &str,
        outgoing: mpsc::Sender<String>,
    ) -> Option<String> {
        let response = match serde_json::from_str::<Value>(line) {
            Ok(value) if value.get("method").is_none() => {
                self.handle_client_response(value);
                None
            }
            Ok(value) => match serde_json::from_value::<IncomingMessage>(value) {
                Ok(message) => self.handle_message(message, outgoing),
                Err(error) => Some(error_response(
                    Value::Null,
                    -32600,
                    format!("invalid request: {error}"),
                )),
            },
            Err(error) => Some(error_response(
                Value::Null,
                -32700,
                format!("parse error: {error}"),
            )),
        };
        response.map(|value| {
            serde_json::to_string(&value).expect("MCP JSON-RPC response must serialize")
        })
    }

    fn handle_message(
        &self,
        message: IncomingMessage,
        outgoing: mpsc::Sender<String>,
    ) -> Option<Value> {
        if message.jsonrpc != "2.0" {
            return message
                .id
                .map(|id| error_response(id_value(&id), -32600, "jsonrpc must be exactly '2.0'"));
        }
        let Some(id) = message.id else {
            self.handle_notification(&message.method, message.params);
            return None;
        };
        if message.method == "initialize" {
            return Some(self.initialize(id, message.params));
        }
        if !self.inner.initialized.load(Ordering::Acquire) {
            return Some(error_response(
                id_value(&id),
                -32001,
                "server has not completed initialize",
            ));
        }
        if message.method == "tools/call" {
            return match self.call_tool(&id, message.params, outgoing) {
                Ok(Some(result)) => Some(success_response(id_value(&id), result)),
                Ok(None) => None,
                Err((code, message)) => Some(error_response(id_value(&id), code, message)),
            };
        }
        let result = match message.method.as_str() {
            "ping" => Ok(json!({})),
            "tools/list" => Ok(tools_result()),
            _ => Err((-32601, format!("method not found: {}", message.method))),
        };
        Some(match result {
            Ok(result) => success_response(id_value(&id), result),
            Err((code, message)) => error_response(id_value(&id), code, message),
        })
    }

    fn initialize(&self, id: JsonRpcId, params: Value) -> Value {
        let params: InitializeParams = match serde_json::from_value(params) {
            Ok(params) => params,
            Err(error) => {
                return error_response(
                    id_value(&id),
                    -32602,
                    format!("invalid initialize params: {error}"),
                );
            }
        };
        if params.protocol_version.trim().is_empty()
            || params.client_info.name.trim().is_empty()
            || params.client_info.version.trim().is_empty()
        {
            return error_response(
                id_value(&id),
                -32602,
                "protocolVersion and clientInfo name/version must not be empty",
            );
        }
        if let Ok(mut features) = self.inner.client_features.lock() {
            features.elicitation_form = supports_form_elicitation(&params.capabilities);
        }
        if self
            .inner
            .initialized
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return error_response(id_value(&id), -32600, "initialize may only be called once");
        }
        success_response(id_value(&id), initialize_result())
    }

    fn handle_notification(&self, method: &str, params: Value) {
        match method {
            "notifications/initialized" => {}
            "notifications/cancelled" => {
                if let Ok(params) = serde_json::from_value::<CancelledParams>(params) {
                    self.cancel(&params.request_id);
                }
            }
            _ => {}
        }
    }

    fn call_tool(
        &self,
        id: &JsonRpcId,
        params: Value,
        outgoing: mpsc::Sender<String>,
    ) -> Result<Option<Value>, (i64, String)> {
        let params: CallToolParams = serde_json::from_value(params)
            .map_err(|error| (-32602, format!("invalid tools/call params: {error}")))?;
        let progress_token = params
            .progress_token()
            .map_err(|message| (-32602, message))?;
        let cancellation = self.register_request(id)?;
        let events =
            McpAgentEvents::new(self.clone(), outgoing, progress_token, cancellation.clone());
        let result = match params.name.as_str() {
            TOOL_START => decode_start(params.arguments)
                .map_err(AgentCallError::InvalidInput)
                .and_then(|request| self.inner.agent.start(request, &cancellation, &events)),
            TOOL_REPLY => decode_reply(params.arguments)
                .map_err(AgentCallError::InvalidInput)
                .and_then(|request| self.inner.agent.reply(request, &cancellation, &events)),
            _ => Err(AgentCallError::InvalidInput(format!(
                "unknown tool '{}'",
                params.name
            ))),
        };
        let cancelled = cancellation.load(Ordering::Acquire);
        self.unregister_request(id);
        if cancelled {
            return Ok(None);
        }
        Ok(Some(match result {
            Ok(outcome) => outcome_result(outcome),
            Err(error) => tool_error(error.to_string()),
        }))
    }

    fn handle_client_response(&self, value: Value) {
        if value.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return;
        }
        let Some(id) = value
            .get("id")
            .cloned()
            .and_then(|id| serde_json::from_value::<JsonRpcId>(id).ok())
        else {
            return;
        };
        let result = if let Some(result) = value.get("result") {
            Ok(result.clone())
        } else if let Some(error) = value.get("error") {
            Err(error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("client returned an elicitation error")
                .to_string())
        } else {
            return;
        };
        if let Ok(mut pending) = self.inner.pending_server_requests.lock()
            && let Some(sender) = pending.remove(&id)
        {
            let _ = sender.send(result);
        }
    }

    fn register_request(&self, id: &JsonRpcId) -> Result<Arc<AtomicBool>, (i64, String)> {
        let cancellation = Arc::new(AtomicBool::new(
            self.inner.shutting_down.load(Ordering::Acquire),
        ));
        if self
            .inner
            .early_cancellations
            .lock()
            .map_err(|_| (-32603, "cancellation lock poisoned".into()))?
            .remove(id)
        {
            cancellation.store(true, Ordering::Release);
        }
        let mut active = self
            .inner
            .active_requests
            .lock()
            .map_err(|_| (-32603, "active request lock poisoned".into()))?;
        if active.contains_key(id) {
            return Err((-32600, "request ID is already active".into()));
        }
        active.insert(id.clone(), cancellation.clone());
        Ok(cancellation)
    }

    fn unregister_request(&self, id: &JsonRpcId) {
        if let Ok(mut active) = self.inner.active_requests.lock() {
            active.remove(id);
        }
    }

    fn cancel(&self, id: &JsonRpcId) {
        if let Ok(active) = self.inner.active_requests.lock()
            && let Some(cancellation) = active.get(id)
        {
            cancellation.store(true, Ordering::Release);
            return;
        }
        if let Ok(mut early) = self.inner.early_cancellations.lock()
            && early.len() < MAX_EARLY_CANCELLATIONS
        {
            early.insert(id.clone());
        }
    }

    pub(crate) fn shutdown(&self) {
        self.inner.shutting_down.store(true, Ordering::Release);
        if let Ok(active) = self.inner.active_requests.lock() {
            for cancellation in active.values() {
                cancellation.store(true, Ordering::Release);
            }
        }
    }
}

fn supports_form_elicitation(capabilities: &Value) -> bool {
    capabilities
        .get("elicitation")
        .and_then(Value::as_object)
        .is_some_and(|elicitation| elicitation.is_empty() || elicitation.contains_key("form"))
}

pub(crate) fn serve_stdio(agent: Arc<dyn AgentService>) -> Result<(), McpServerError> {
    serve_connection(
        BufReader::new(std::io::stdin()),
        std::io::stdout(),
        McpServer::new(agent),
    )
}

fn serve_connection<R: BufRead, W: Write + Send + 'static>(
    reader: R,
    mut writer: W,
    server: McpServer,
) -> Result<(), McpServerError> {
    let (outgoing_tx, outgoing_rx) = mpsc::channel::<String>();
    let writer_thread = thread::spawn(move || -> Result<(), std::io::Error> {
        for message in outgoing_rx {
            writeln!(writer, "{message}")?;
            writer.flush()?;
        }
        Ok(())
    });
    let mut workers = Vec::new();
    let mut connection_result = (|| {
        for line in reader.lines() {
            let line = line.map_err(McpServerError::io)?;
            reap_finished(&mut workers)?;
            let worker_server = server.clone();
            let worker_tx = outgoing_tx.clone();
            workers.push(thread::spawn(move || {
                if let Some(response) =
                    worker_server.handle_line_with_outgoing(&line, worker_tx.clone())
                {
                    let _ = worker_tx.send(response);
                }
            }));
        }
        Ok(())
    })();
    server.shutdown();
    for worker in workers {
        if worker.join().is_err() && connection_result.is_ok() {
            connection_result = Err(McpServerError("MCP request worker panicked".into()));
        }
    }
    drop(outgoing_tx);
    let writer_result = writer_thread
        .join()
        .map_err(|_| McpServerError("MCP stdio writer panicked".into()))?
        .map_err(McpServerError::io);
    connection_result.and(writer_result)
}

fn reap_finished(workers: &mut Vec<thread::JoinHandle<()>>) -> Result<(), McpServerError> {
    let mut index = 0;
    while index < workers.len() {
        if workers[index].is_finished() {
            workers
                .remove(index)
                .join()
                .map_err(|_| McpServerError("MCP request worker panicked".into()))?;
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn outcome_result(mut outcome: AgentOutcome) -> Value {
    outcome.content = truncate_utf8(outcome.content, MAX_TOOL_RESULT_BYTES);
    let is_error = outcome.is_error();
    let content = outcome.content.clone();
    json!({
        "content": [
            {
                "type": "text",
                "text": content
            }
        ],
        "structuredContent": outcome,
        "isError": is_error
    })
}

fn truncate_utf8(mut value: String, maximum_bytes: usize) -> String {
    const MARKER: &str = "\n[output truncated]";
    if value.len() <= maximum_bytes {
        return value;
    }
    let mut boundary = maximum_bytes.saturating_sub(MARKER.len());
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value.push_str(MARKER);
    value
}

fn tool_error(message: String) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": message
            }
        ],
        "isError": true
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn error_response(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

fn id_value(id: &JsonRpcId) -> Value {
    serde_json::to_value(id).expect("JSON-RPC ID must serialize")
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
