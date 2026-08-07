mod wire;

use crate::CollaborationServerError;
use crate::CollaborationServerOptions;
use serde::Serialize;
use serde_json::json;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use wire::HttpReadError;
use wire::HttpRequest;
use wire::authorized;
use wire::read_request;
use wire::write_empty_response;
use wire::write_json_response;
use zeta_collaboration::DocumentCollaborationOpenParams;
use zeta_collaboration::DocumentCollaborationReplay;
use zeta_collaboration::DocumentCollaborationSnapshot;
use zeta_collaboration::DocumentCollaborationSubmitParams;
use zeta_collaboration::DocumentCollaborationSubmitResult;
use zeta_collaboration::DocumentCollaborationUpdate;
use zeta_collaboration::SqliteDocumentCollaborationRooms;

const API_ROOT: &str = "/v1/document-collaboration";
const OPEN_PATH: &str = "/v1/document-collaboration/rooms/open";
const SUBMIT_PATH: &str = "/v1/document-collaboration/rooms/submit";
const POLL_TIMEOUT: Duration = Duration::from_secs(25);

pub(crate) fn serve(options: CollaborationServerOptions) -> Result<(), CollaborationServerError> {
    let listener =
        TcpListener::bind(options.listen_address()).map_err(CollaborationServerError::http)?;
    let rooms = SqliteDocumentCollaborationRooms::open_at(options.database_path())
        .map_err(CollaborationServerError::storage)?;
    let runtime = Arc::new(HttpRuntime::new(rooms, options));
    serve_listener(listener, runtime, Arc::new(AtomicBool::new(false)))
}

struct HttpRuntime {
    rooms: SqliteDocumentCollaborationRooms,
    options: CollaborationServerOptions,
    active_connections: AtomicUsize,
    updates: UpdateSignal,
}

impl HttpRuntime {
    fn new(rooms: SqliteDocumentCollaborationRooms, options: CollaborationServerOptions) -> Self {
        Self {
            rooms,
            options,
            active_connections: AtomicUsize::new(0),
            updates: UpdateSignal::default(),
        }
    }
}

#[derive(Default)]
struct UpdateSignal {
    generation: Mutex<u64>,
    changed: Condvar,
}

impl UpdateSignal {
    fn current(&self) -> u64 {
        self.generation
            .lock()
            .map(|generation| *generation)
            .unwrap_or_default()
    }

    fn notify(&self) {
        if let Ok(mut generation) = self.generation.lock() {
            *generation = generation.wrapping_add(1);
            self.changed.notify_all();
        }
    }

    fn wait_for_change(&self, generation: u64) {
        let Ok(guard) = self.generation.lock() else {
            return;
        };
        if *guard == generation {
            let _ = self.changed.wait_timeout(guard, POLL_TIMEOUT);
        }
    }
}

fn serve_listener(
    listener: TcpListener,
    runtime: Arc<HttpRuntime>,
    shutdown: Arc<AtomicBool>,
) -> Result<(), CollaborationServerError> {
    listener
        .set_nonblocking(true)
        .map_err(CollaborationServerError::http)?;
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
                thread::sleep(Duration::from_millis(10))
            }
            Err(error) => return Err(CollaborationServerError::http(error)),
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
    runtime: &HttpRuntime,
) -> Result<(), CollaborationServerError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(CollaborationServerError::http)?;
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(HttpReadError::Status(status, reason)) => {
            return write_empty_response(&mut stream, status, reason, &[])
                .map_err(CollaborationServerError::http);
        }
        Err(HttpReadError::Io(error)) => return Err(CollaborationServerError::http(error)),
    };
    let (path, _) = split_target(&request.target);
    if !path.starts_with(API_ROOT) {
        return write_empty_response(&mut stream, 404, "Not Found", &[])
            .map_err(CollaborationServerError::http);
    }
    if !origin_allowed(&request, runtime) {
        return write_empty_response(&mut stream, 403, "Forbidden", &[])
            .map_err(CollaborationServerError::http);
    }
    if request.method == "OPTIONS" {
        return write_empty_response(
            &mut stream,
            204,
            "No Content",
            &cors_preflight_headers(&request),
        )
        .map_err(CollaborationServerError::http);
    }
    if !authorized(&request, runtime.options.bearer_token()) {
        let mut headers = cors_headers(&request);
        headers.push(("WWW-Authenticate", "Bearer"));
        return write_empty_response(&mut stream, 401, "Unauthorized", &headers)
            .map_err(CollaborationServerError::http);
    }
    match (request.method.as_str(), path) {
        ("POST", OPEN_PATH) => handle_open(&mut stream, runtime, &request),
        ("POST", SUBMIT_PATH) => handle_submit(&mut stream, runtime, &request),
        ("GET", _) => handle_updates(&mut stream, runtime, &request),
        _ => {
            let mut headers = cors_headers(&request);
            headers.push(("Allow", "GET, POST, OPTIONS"));
            write_empty_response(&mut stream, 405, "Method Not Allowed", &headers)
                .map_err(CollaborationServerError::http)
        }
    }
}

fn handle_open(
    stream: &mut TcpStream,
    runtime: &HttpRuntime,
    request: &HttpRequest,
) -> Result<(), CollaborationServerError> {
    let Some(params) = decode_json::<DocumentCollaborationOpenParams>(request) else {
        return write_error(
            stream,
            runtime,
            request,
            400,
            "Bad Request",
            "Expected a valid collaboration room request",
        );
    };
    match runtime.rooms.open(params) {
        Ok(result) => write_json(stream, runtime, request, 200, "OK", &result),
        Err(error) => write_domain_error(stream, runtime, request, error),
    }
}

fn handle_submit(
    stream: &mut TcpStream,
    runtime: &HttpRuntime,
    request: &HttpRequest,
) -> Result<(), CollaborationServerError> {
    let Some(params) = decode_json::<DocumentCollaborationSubmitParams>(request) else {
        return write_error(
            stream,
            runtime,
            request,
            400,
            "Bad Request",
            "Expected a valid collaboration update",
        );
    };
    match runtime.rooms.submit(params) {
        Ok(result) => {
            if matches!(result, DocumentCollaborationSubmitResult::Accepted { .. }) {
                runtime.updates.notify();
            }
            write_json(stream, runtime, request, 200, "OK", &result)
        }
        Err(error) => write_domain_error(stream, runtime, request, error),
    }
}

fn handle_updates(
    stream: &mut TcpStream,
    runtime: &HttpRuntime,
    request: &HttpRequest,
) -> Result<(), CollaborationServerError> {
    let (path, query) = split_target(&request.target);
    let Some(room_id) = path
        .strip_prefix(&format!("{API_ROOT}/rooms/"))
        .and_then(|value| value.strip_suffix("/updates"))
    else {
        return write_empty_response(stream, 404, "Not Found", &cors_headers(request))
            .map_err(CollaborationServerError::http);
    };
    let Some(after_version) = query.and_then(after_version) else {
        return write_error(
            stream,
            runtime,
            request,
            400,
            "Bad Request",
            "afterVersion must be a non-negative safe integer",
        );
    };
    let generation = runtime.updates.current();
    let replay = match runtime.rooms.replay(room_id, after_version) {
        Ok(replay) => replay,
        Err(error) => return write_domain_error(stream, runtime, request, error),
    };
    if matches!(replay, DocumentCollaborationReplay::Updates(ref updates) if updates.is_empty()) {
        runtime.updates.wait_for_change(generation);
    }
    match runtime.rooms.replay(room_id, after_version) {
        Ok(replay) => write_json(
            stream,
            runtime,
            request,
            200,
            "OK",
            &RemoteReplay::from(replay),
        ),
        Err(error) => write_domain_error(stream, runtime, request, error),
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum RemoteReplay {
    Updates {
        updates: Vec<DocumentCollaborationUpdate>,
    },
    Resync {
        snapshot: DocumentCollaborationSnapshot,
    },
}

impl From<DocumentCollaborationReplay> for RemoteReplay {
    fn from(value: DocumentCollaborationReplay) -> Self {
        match value {
            DocumentCollaborationReplay::Updates(updates) => Self::Updates { updates },
            DocumentCollaborationReplay::Resync(snapshot) => Self::Resync { snapshot },
        }
    }
}

fn decode_json<T: serde::de::DeserializeOwned>(request: &HttpRequest) -> Option<T> {
    if !request
        .header("content-type")
        .is_some_and(|value| value.starts_with("application/json"))
    {
        return None;
    }
    serde_json::from_slice(&request.body).ok()
}

fn split_target(target: &str) -> (&str, Option<&str>) {
    target
        .split_once('?')
        .map_or((target, None), |(path, query)| (path, Some(query)))
}

fn after_version(query: &str) -> Option<u64> {
    let mut value = None;
    for pair in query.split('&') {
        let (name, candidate) = pair.split_once('=')?;
        if name != "afterVersion" || value.is_some() {
            return None;
        }
        let parsed = candidate.parse::<u64>().ok()?;
        if parsed > 9_007_199_254_740_991 {
            return None;
        }
        value = Some(parsed);
    }
    value
}

fn origin_allowed(request: &HttpRequest, runtime: &HttpRuntime) -> bool {
    request
        .header("origin")
        .is_none_or(|origin| runtime.options.allowed_origins().contains(origin))
}

fn cors_headers(request: &HttpRequest) -> Vec<(&str, &str)> {
    request
        .header("origin")
        .map(|origin| vec![("Access-Control-Allow-Origin", origin)])
        .unwrap_or_default()
}

fn cors_preflight_headers(request: &HttpRequest) -> Vec<(&str, &str)> {
    let mut headers = cors_headers(request);
    if !headers.is_empty() {
        headers.push((
            "Access-Control-Allow-Headers",
            "authorization, content-type",
        ));
        headers.push(("Access-Control-Allow-Methods", "GET, POST, OPTIONS"));
        headers.push(("Access-Control-Max-Age", "600"));
    }
    headers
}

fn write_json<T: Serialize>(
    stream: &mut TcpStream,
    _runtime: &HttpRuntime,
    request: &HttpRequest,
    status: u16,
    reason: &str,
    value: &T,
) -> Result<(), CollaborationServerError> {
    let body = serde_json::to_vec(value).map_err(|error| {
        CollaborationServerError::storage(format!(
            "Could not serialize collaboration response: {error}"
        ))
    })?;
    write_json_response(stream, status, reason, &body, &cors_headers(request))
        .map_err(CollaborationServerError::http)
}

fn write_error(
    stream: &mut TcpStream,
    runtime: &HttpRuntime,
    request: &HttpRequest,
    status: u16,
    reason: &str,
    message: &str,
) -> Result<(), CollaborationServerError> {
    write_json(
        stream,
        runtime,
        request,
        status,
        reason,
        &json!({ "error": message }),
    )
}

fn write_domain_error(
    stream: &mut TcpStream,
    runtime: &HttpRuntime,
    request: &HttpRequest,
    error: String,
) -> Result<(), CollaborationServerError> {
    if error.starts_with("Collaboration database error")
        || error == "Collaboration database lock poisoned"
    {
        return write_error(
            stream,
            runtime,
            request,
            500,
            "Internal Server Error",
            "The collaboration server could not process the request",
        );
    }
    let (status, reason) = if error.contains("does not exist") {
        (404, "Not Found")
    } else {
        (422, "Unprocessable Content")
    };
    write_error(stream, runtime, request, status, reason, &error)
}

#[cfg(test)]
#[path = "http/tests.rs"]
mod tests;
