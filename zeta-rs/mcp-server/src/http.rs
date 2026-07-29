mod wire;

use crate::agent::{AgentService, AppServerAgentService, RuntimeLimits};
use crate::options::HttpServerOptions;
use crate::protocol::MCP_PROTOCOL_VERSION;
use crate::receipt::ReceiptStore;
use crate::server::{McpServer, McpServerError};
use getrandom::getrandom;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use wire::{
    HttpReadError, HttpRequest, authorized, origin_allowed, read_request, write_empty_response,
    write_json_response, write_sse_event, write_sse_headers,
};
use zeta_app_server_client::InProcessAppServer;

const SESSION_HEADER: &str = "mcp-session-id";
const PROTOCOL_HEADER: &str = "mcp-protocol-version";

pub(crate) fn serve(
    host: InProcessAppServer,
    limits: RuntimeLimits,
    receipts: Arc<ReceiptStore>,
    options: HttpServerOptions,
) -> Result<(), McpServerError> {
    let listener = TcpListener::bind(options.listen_address()).map_err(McpServerError::http)?;
    let runtime = Arc::new(HttpRuntime::new(host, limits, receipts, options));
    serve_listener(listener, runtime, Arc::new(AtomicBool::new(false)))
}

struct HttpRuntime {
    host: InProcessAppServer,
    limits: RuntimeLimits,
    receipts: Arc<ReceiptStore>,
    options: HttpServerOptions,
    sessions: Mutex<BTreeMap<String, McpServer>>,
    active_connections: AtomicUsize,
    next_stream_id: AtomicU64,
    principal: String,
}

impl HttpRuntime {
    fn new(
        host: InProcessAppServer,
        limits: RuntimeLimits,
        receipts: Arc<ReceiptStore>,
        options: HttpServerOptions,
    ) -> Self {
        let principal = format!(
            "http:{}",
            hex_digest(Sha256::digest(options.bearer_token().as_bytes()).as_slice())
        );
        Self {
            host,
            limits,
            receipts,
            options,
            sessions: Mutex::new(BTreeMap::new()),
            active_connections: AtomicUsize::new(0),
            next_stream_id: AtomicU64::new(1),
            principal,
        }
    }

    fn create_session(&self) -> Result<(String, McpServer), McpServerError> {
        let session_id = secure_session_id()?;
        let client = self.host.connect().map_err(McpServerError::app_server)?;
        let agent: Arc<dyn AgentService> = Arc::new(AppServerAgentService::with_receipts(
            client,
            self.limits,
            self.receipts.clone(),
            self.principal.clone(),
        ));
        Ok((session_id, McpServer::new(agent)))
    }

    fn session(&self, session_id: &str) -> Option<McpServer> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(session_id).cloned())
    }
}

fn serve_listener(
    listener: TcpListener,
    runtime: Arc<HttpRuntime>,
    shutdown: Arc<AtomicBool>,
) -> Result<(), McpServerError> {
    listener
        .set_nonblocking(true)
        .map_err(McpServerError::http)?;
    while !shutdown.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                let active = runtime.active_connections.fetch_add(1, Ordering::AcqRel) + 1;
                if active > runtime.options.maximum_connections() {
                    runtime.active_connections.fetch_sub(1, Ordering::AcqRel);
                    let mut stream = stream;
                    let _ = write_empty_response(&mut stream, 503, "Service Unavailable", &[]);
                    continue;
                }
                let connection_runtime = runtime.clone();
                thread::spawn(move || {
                    let _guard = ConnectionGuard(&connection_runtime.active_connections);
                    let _ = handle_connection(stream, &connection_runtime);
                });
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(McpServerError::http(error)),
        }
    }
    Ok(())
}

struct ConnectionGuard<'a>(&'a AtomicUsize);

impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

fn handle_connection(
    mut stream: TcpStream,
    runtime: &Arc<HttpRuntime>,
) -> Result<(), McpServerError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(McpServerError::http)?;
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(HttpReadError::Status(status, reason)) => {
            return write_empty_response(&mut stream, status, reason, &[])
                .map_err(McpServerError::http);
        }
        Err(HttpReadError::Io(error)) => return Err(McpServerError::http(error)),
    };
    if request.path != runtime.options.endpoint_path() {
        return write_empty_response(&mut stream, 404, "Not Found", &[])
            .map_err(McpServerError::http);
    }
    if !authorized(&request, runtime.options.bearer_token()) {
        return write_empty_response(
            &mut stream,
            401,
            "Unauthorized",
            &[("WWW-Authenticate", "Bearer")],
        )
        .map_err(McpServerError::http);
    }
    if !origin_allowed(&request, runtime.options.allowed_origins()) {
        return write_empty_response(&mut stream, 403, "Forbidden", &[])
            .map_err(McpServerError::http);
    }
    match request.method.as_str() {
        "POST" => handle_post(stream, runtime, request),
        "GET" => write_empty_response(
            &mut stream,
            405,
            "Method Not Allowed",
            &[("Allow", "POST, DELETE")],
        )
        .map_err(McpServerError::http),
        "DELETE" => handle_delete(&mut stream, runtime, &request),
        _ => write_empty_response(
            &mut stream,
            405,
            "Method Not Allowed",
            &[("Allow", "POST, DELETE")],
        )
        .map_err(McpServerError::http),
    }
}

fn handle_post(
    mut stream: TcpStream,
    runtime: &Arc<HttpRuntime>,
    request: HttpRequest,
) -> Result<(), McpServerError> {
    let accept = request.header("accept").unwrap_or_default();
    if !accept.contains("application/json") || !accept.contains("text/event-stream") {
        return write_empty_response(&mut stream, 406, "Not Acceptable", &[])
            .map_err(McpServerError::http);
    }
    if !request
        .header("content-type")
        .is_some_and(|value| value.starts_with("application/json"))
    {
        return write_empty_response(&mut stream, 415, "Unsupported Media Type", &[])
            .map_err(McpServerError::http);
    }
    let value: Value = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(_) => {
            return write_empty_response(&mut stream, 400, "Bad Request", &[])
                .map_err(McpServerError::http);
        }
    };
    let method = value.get("method").and_then(Value::as_str);
    let is_initialize = method == Some("initialize");
    let supplied_session = request.header(SESSION_HEADER);

    let (session_id, server, created) = if is_initialize && supplied_session.is_none() {
        let (session_id, server) = runtime.create_session()?;
        (session_id, server, true)
    } else {
        let Some(session_id) = supplied_session else {
            return write_empty_response(&mut stream, 400, "Bad Request", &[])
                .map_err(McpServerError::http);
        };
        if request.header(PROTOCOL_HEADER) != Some(MCP_PROTOCOL_VERSION) {
            return write_empty_response(&mut stream, 400, "Bad Request", &[])
                .map_err(McpServerError::http);
        }
        let Some(server) = runtime.session(session_id) else {
            return write_empty_response(&mut stream, 404, "Not Found", &[])
                .map_err(McpServerError::http);
        };
        (session_id.to_string(), server, false)
    };

    let encoded = serde_json::to_string(&value).map_err(McpServerError::http)?;
    let is_request = method.is_some() && value.get("id").is_some();
    if !is_request {
        let (outgoing, _) = mpsc::channel();
        let _ = server.handle_line_with_outgoing(&encoded, outgoing);
        return write_empty_response(
            &mut stream,
            202,
            "Accepted",
            &[("MCP-Session-Id", &session_id)],
        )
        .map_err(McpServerError::http);
    }
    if method == Some("tools/call") {
        return stream_tool_call(stream, runtime, server, encoded, &session_id);
    }
    let (outgoing, _) = mpsc::channel();
    let Some(response) = server.handle_line_with_outgoing(&encoded, outgoing) else {
        return write_empty_response(
            &mut stream,
            202,
            "Accepted",
            &[("MCP-Session-Id", &session_id)],
        )
        .map_err(McpServerError::http);
    };
    let successful_initialize = created
        && serde_json::from_str::<Value>(&response)
            .ok()
            .is_some_and(|value| value.get("result").is_some());
    if successful_initialize {
        runtime
            .sessions
            .lock()
            .map_err(|_| McpServerError::http("HTTP session lock poisoned"))?
            .insert(session_id.clone(), server);
    }
    let headers = if successful_initialize {
        vec![("MCP-Session-Id", session_id.as_str())]
    } else {
        Vec::new()
    };
    write_json_response(&mut stream, 200, "OK", response.as_bytes(), &headers)
        .map_err(McpServerError::http)
}

fn stream_tool_call(
    mut stream: TcpStream,
    runtime: &HttpRuntime,
    server: McpServer,
    encoded: String,
    session_id: &str,
) -> Result<(), McpServerError> {
    write_sse_headers(&mut stream, session_id).map_err(McpServerError::http)?;
    let stream_id = runtime.next_stream_id.fetch_add(1, Ordering::Relaxed);
    let (outgoing, incoming) = mpsc::channel();
    let worker_server = server.clone();
    let worker_outgoing = outgoing.clone();
    let worker = thread::spawn(move || {
        if let Some(response) =
            worker_server.handle_line_with_outgoing(&encoded, worker_outgoing.clone())
        {
            let _ = worker_outgoing.send(response);
        }
    });
    drop(outgoing);
    for (sequence, message) in (1_u64..).zip(incoming) {
        if let Err(error) =
            write_sse_event(&mut stream, &format!("{stream_id}:{sequence}"), &message)
        {
            return Err(McpServerError::http(error));
        }
    }
    worker
        .join()
        .map_err(|_| McpServerError::http("HTTP MCP request worker panicked"))
}

fn handle_delete(
    stream: &mut TcpStream,
    runtime: &HttpRuntime,
    request: &HttpRequest,
) -> Result<(), McpServerError> {
    let Some(session_id) = request.header(SESSION_HEADER) else {
        return write_empty_response(stream, 400, "Bad Request", &[]).map_err(McpServerError::http);
    };
    if request.header(PROTOCOL_HEADER) != Some(MCP_PROTOCOL_VERSION) {
        return write_empty_response(stream, 400, "Bad Request", &[]).map_err(McpServerError::http);
    }
    let removed = runtime
        .sessions
        .lock()
        .map_err(|_| McpServerError::http("HTTP session lock poisoned"))?
        .remove(session_id);
    let Some(server) = removed else {
        return write_empty_response(stream, 404, "Not Found", &[]).map_err(McpServerError::http);
    };
    server.shutdown();
    write_empty_response(stream, 204, "No Content", &[]).map_err(McpServerError::http)
}

fn secure_session_id() -> Result<String, McpServerError> {
    let mut bytes = [0_u8; 32];
    getrandom(&mut bytes).map_err(McpServerError::http)?;
    Ok(hex_digest(&bytes))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
