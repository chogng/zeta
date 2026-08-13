use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::BufReader;
use std::path::Path;
use std::process::Child;
use std::process::ChildStdin;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::TrySendError;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;
use zeta_app_server_transport::JsonlReader;
use zeta_app_server_transport::JsonlWriter;

const MAX_UPSTREAM_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
const EVENT_QUEUE_CAPACITY: usize = 256;

type PendingSender = SyncSender<Result<Value, ProcessError>>;
type PendingRequests = Arc<Mutex<BTreeMap<u64, PendingSender>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessErrorKind {
    Unavailable,
    Unsupported,
    Rejected,
}

#[derive(Clone, Debug)]
pub(crate) struct ProcessError {
    pub(crate) kind: ProcessErrorKind,
    pub(crate) message: &'static str,
}

impl ProcessError {
    pub(crate) fn unavailable(message: &'static str) -> Self {
        Self {
            kind: ProcessErrorKind::Unavailable,
            message,
        }
    }

    fn unsupported() -> Self {
        Self {
            kind: ProcessErrorKind::Unsupported,
            message: "installed Codex App Server does not support the required method",
        }
    }

    fn rejected() -> Self {
        Self {
            kind: ProcessErrorKind::Rejected,
            message: "Codex App Server rejected the account operation",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum UpstreamEvent {
    Notification {
        method: String,
        params: Value,
    },
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    ConnectionClosed,
}

pub(crate) struct CodexAppServerProcess {
    child: Mutex<Child>,
    writer: Mutex<JsonlWriter<ChildStdin>>,
    pending: PendingRequests,
    next_request_id: AtomicU64,
    request_timeout: Duration,
}

enum RequestParameters {
    Included(Value),
    Omitted,
}

impl CodexAppServerProcess {
    pub(crate) fn start(
        program: &Path,
        request_timeout: Duration,
    ) -> Result<(Arc<Self>, Receiver<UpstreamEvent>), ProcessError> {
        let mut child = Command::new(program)
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ProcessError::unavailable("could not start the local Codex App Server"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProcessError::unavailable("Codex App Server stdin was unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProcessError::unavailable("Codex App Server stdout was unavailable"))?;
        let pending = Arc::new(Mutex::new(BTreeMap::new()));
        let (events, receiver) = sync_channel(EVENT_QUEUE_CAPACITY);
        let reader_pending = Arc::clone(&pending);
        if thread::Builder::new()
            .name("zeta-codex-app-server-reader".into())
            .spawn(move || read_output(stdout, reader_pending, events))
            .is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProcessError::unavailable(
                "could not start the Codex App Server reader",
            ));
        }
        let process = Arc::new(Self {
            child: Mutex::new(child),
            writer: Mutex::new(JsonlWriter::new(stdin, MAX_UPSTREAM_MESSAGE_BYTES)),
            pending,
            next_request_id: AtomicU64::new(1),
            request_timeout,
        });
        let initialized = process.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "zeta",
                    "title": "Zeta",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "experimentalApi": true,
                    "requestAttestation": false,
                    "mcpServerOpenaiFormElicitation": false
                }
            }),
        )?;
        if initialized
            .get("userAgent")
            .and_then(Value::as_str)
            .is_none()
        {
            return Err(ProcessError::unsupported());
        }
        process.notify("initialized", None)?;
        Ok((process, receiver))
    }

    pub(crate) fn request(&self, method: &str, params: Value) -> Result<Value, ProcessError> {
        self.request_with(method, RequestParameters::Included(params))
    }

    pub(crate) fn request_without_params(&self, method: &str) -> Result<Value, ProcessError> {
        self.request_with(method, RequestParameters::Omitted)
    }

    pub(crate) fn respond(&self, id: Value, result: Value) -> Result<(), ProcessError> {
        self.write_value(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    pub(crate) fn respond_error(
        &self,
        id: Value,
        code: i64,
        message: &'static str,
    ) -> Result<(), ProcessError> {
        self.write_value(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message,
            },
        }))
    }

    fn request_with(&self, method: &str, params: RequestParameters) -> Result<Value, ProcessError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = sync_channel(1);
        self.pending
            .lock()
            .map_err(|_| ProcessError::unavailable("Codex request state was unavailable"))?
            .insert(request_id, sender);
        let mut request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
        });
        if let RequestParameters::Included(params) = params {
            request["params"] = params;
        }
        let write_result = self.write_value(request);
        if let Err(error) = write_result {
            self.remove_pending(request_id);
            return Err(error);
        }
        match receiver.recv_timeout(self.request_timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => {
                self.remove_pending(request_id);
                Err(ProcessError::unavailable(
                    "Codex App Server request timed out",
                ))
            }
            Err(RecvTimeoutError::Disconnected) => Err(ProcessError::unavailable(
                "Codex App Server closed the request channel",
            )),
        }
    }

    fn notify(&self, method: &str, params: Option<Value>) -> Result<(), ProcessError> {
        let mut message = json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(params) = params {
            message["params"] = params;
        }
        self.write_value(message)
    }

    fn write_value(&self, message: Value) -> Result<(), ProcessError> {
        let message = serde_json::to_string(&message)
            .map_err(|_| ProcessError::unavailable("could not encode the Codex message"))?;
        self.writer
            .lock()
            .map_err(|_| ProcessError::unavailable("Codex writer state was unavailable"))?
            .write_message(&message)
            .map_err(|_| ProcessError::unavailable("could not write to the Codex App Server"))
    }

    fn remove_pending(&self, request_id: u64) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&request_id);
        }
    }
}

impl Drop for CodexAppServerProcess {
    fn drop(&mut self) {
        if let Ok(child) = self.child.get_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn read_output(
    stdout: std::process::ChildStdout,
    pending: PendingRequests,
    events: SyncSender<UpstreamEvent>,
) {
    let mut reader = JsonlReader::new(BufReader::new(stdout), MAX_UPSTREAM_MESSAGE_BYTES);
    loop {
        let message = match reader.read_message() {
            Ok(Some(message)) => message,
            Ok(None) | Err(_) => {
                fail_pending(&pending);
                return;
            }
        };
        let value: Value = match serde_json::from_str(&message) {
            Ok(value) => value,
            Err(_) => {
                fail_pending(&pending);
                return;
            }
        };
        if let Some(method) = value.get("method").and_then(Value::as_str) {
            let params = value.get("params").cloned().unwrap_or(Value::Null);
            let event = match value.get("id") {
                Some(id) => UpstreamEvent::Request {
                    id: id.clone(),
                    method: method.to_owned(),
                    params,
                },
                None => UpstreamEvent::Notification {
                    method: method.to_owned(),
                    params,
                },
            };
            match events.try_send(event) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                    fail_pending(&pending);
                    return;
                }
            }
            continue;
        }
        if let Some(request_id) = value.get("id").and_then(Value::as_u64) {
            let sender = pending
                .lock()
                .ok()
                .and_then(|mut pending| pending.remove(&request_id));
            if let Some(sender) = sender {
                let _ = sender.send(decode_response(value));
            }
            continue;
        }
    }
}

fn decode_response(value: Value) -> Result<Value, ProcessError> {
    if let Some(result) = value.get("result") {
        return Ok(result.clone());
    }
    if value
        .get("error")
        .and_then(|error| error.get("code"))
        .and_then(Value::as_i64)
        == Some(-32601)
    {
        return Err(ProcessError::unsupported());
    }
    Err(ProcessError::rejected())
}

fn fail_pending(pending: &PendingRequests) {
    let senders = pending
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default();
    for (_, sender) in senders {
        let _ = sender.send(Err(ProcessError::unavailable(
            "Codex App Server closed its output stream",
        )));
    }
}
