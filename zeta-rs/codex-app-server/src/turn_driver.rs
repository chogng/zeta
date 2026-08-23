use crate::CodexAppServerRuntime;
use crate::process::ProcessError;
use crate::process::ProcessErrorKind;
use crate::process::UpstreamEvent;
use crate::runtime::EventHandling;
use crate::runtime::UpstreamConnectionId;
use crate::runtime::UpstreamEventHandler;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::sync_channel;

const TURN_EVENT_QUEUE_CAPACITY: usize = 256;

mod server_requests;

pub use server_requests::CodexApprovalDecision;
pub use server_requests::CodexCommandApprovalRequest;
pub use server_requests::CodexFileChangeApprovalRequest;
pub use server_requests::CodexServerRequestId;
pub use server_requests::CodexUserInputAnswers;
pub use server_requests::CodexUserInputOption;
pub use server_requests::CodexUserInputQuestion;
pub use server_requests::CodexUserInputRequest;

use server_requests::PendingServerRequestKey;
use server_requests::PendingServerRequestKind;
use server_requests::decode_server_request;

/// Opaque upstream Codex thread identity persisted by the Zeta runtime binding.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CodexThreadId(String);

impl CodexThreadId {
    pub fn new(value: impl Into<String>) -> Result<Self, CodexTurnError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CodexTurnError::invalid_input(
                "Codex thread ID must not be empty",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Opaque upstream Codex Turn identity associated with one local Turn.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CodexTurnId(String);

impl CodexTurnId {
    pub fn new(value: impl Into<String>) -> Result<Self, CodexTurnError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CodexTurnError::invalid_input(
                "Codex Turn ID must not be empty",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Sandboxed access granted to one upstream Codex thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexThreadAccess {
    /// Codex may inspect the workspace but cannot mutate it.
    ReadOnly,
    /// Codex may write inside its workspace and must request approval when required.
    WorkspaceWrite,
}

impl CodexThreadAccess {
    fn approval_policy(self) -> &'static str {
        match self {
            Self::ReadOnly => "never",
            Self::WorkspaceWrite => "on-request",
        }
    }

    fn sandbox(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
        }
    }
}

/// Inputs for creating a persistent upstream Codex thread.
pub struct StartCodexThread {
    cwd: String,
    model: Option<String>,
    access: CodexThreadAccess,
}

impl StartCodexThread {
    pub fn read_only(cwd: &Path) -> Result<Self, CodexTurnError> {
        Self::with_access(cwd, CodexThreadAccess::ReadOnly)
    }

    pub fn workspace_write(cwd: &Path) -> Result<Self, CodexTurnError> {
        Self::with_access(cwd, CodexThreadAccess::WorkspaceWrite)
    }

    fn with_access(cwd: &Path, access: CodexThreadAccess) -> Result<Self, CodexTurnError> {
        if !cwd.is_absolute() {
            return Err(CodexTurnError::invalid_input(
                "Codex thread working directory must be absolute",
            ));
        }
        let cwd = cwd.to_str().ok_or_else(|| {
            CodexTurnError::invalid_input("Codex thread working directory must be valid UTF-8")
        })?;
        Ok(Self {
            cwd: cwd.into(),
            model: None,
            access,
        })
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Result<Self, CodexTurnError> {
        let model = model.into();
        if model.trim().is_empty() {
            return Err(CodexTurnError::invalid_input(
                "Codex thread model must not be empty",
            ));
        }
        self.model = Some(model);
        Ok(self)
    }
}

/// Inputs for starting one user Turn on an upstream Codex thread.
pub struct StartCodexTurn {
    pub thread_id: CodexThreadId,
    text: String,
}

impl StartCodexTurn {
    pub fn text(thread_id: CodexThreadId, text: impl Into<String>) -> Result<Self, CodexTurnError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(CodexTurnError::invalid_input(
                "Codex Turn input must not be empty",
            ));
        }
        Ok(Self { thread_id, text })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexTurnStatus {
    Completed,
    Interrupted,
    Failed,
}

/// Typed upstream events consumed by the Core Turn backend adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexTurnEvent {
    Started {
        thread_id: CodexThreadId,
        turn_id: CodexTurnId,
    },
    AgentMessageDelta {
        thread_id: CodexThreadId,
        turn_id: CodexTurnId,
        item_id: String,
        delta: String,
    },
    ReasoningSummaryDelta {
        thread_id: CodexThreadId,
        turn_id: CodexTurnId,
        item_id: String,
        delta: String,
    },
    ReasoningDelta {
        thread_id: CodexThreadId,
        turn_id: CodexTurnId,
        item_id: String,
        delta: String,
    },
    DiffUpdated {
        thread_id: CodexThreadId,
        turn_id: CodexTurnId,
        diff: String,
    },
    CommandApprovalRequested(CodexCommandApprovalRequest),
    FileChangeApprovalRequested(CodexFileChangeApprovalRequest),
    UserInputRequested(CodexUserInputRequest),
    Completed {
        thread_id: CodexThreadId,
        turn_id: CodexTurnId,
        status: CodexTurnStatus,
    },
    ProtocolError {
        method: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexTurnErrorKind {
    InvalidInput,
    Conflict,
    Unavailable,
    Unsupported,
    Rejected,
    IncompatibleResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexTurnError {
    kind: CodexTurnErrorKind,
    message: &'static str,
}

impl CodexTurnError {
    pub fn kind(&self) -> CodexTurnErrorKind {
        self.kind
    }

    fn invalid_input(message: &'static str) -> Self {
        Self {
            kind: CodexTurnErrorKind::InvalidInput,
            message,
        }
    }

    fn incompatible(message: &'static str) -> Self {
        Self {
            kind: CodexTurnErrorKind::IncompatibleResponse,
            message,
        }
    }

    fn conflict(message: &'static str) -> Self {
        Self {
            kind: CodexTurnErrorKind::Conflict,
            message,
        }
    }
}

impl fmt::Display for CodexTurnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for CodexTurnError {}

/// Upstream thread/Turn client used by the Core execution backend.
///
/// Server requests are projected as typed events and retained until the caller
/// resolves their opaque request ID. Each request can be resolved at most once,
/// and a response is never replayed onto a replacement upstream connection.
pub struct CodexTurnDriver {
    runtime: Arc<CodexAppServerRuntime>,
    events: SyncSender<CodexTurnEvent>,
    pending_requests: Mutex<BTreeMap<PendingServerRequestKey, PendingServerRequestKind>>,
}

impl CodexTurnDriver {
    pub fn new(runtime: Arc<CodexAppServerRuntime>) -> (Arc<Self>, Receiver<CodexTurnEvent>) {
        let (events, receiver) = sync_channel(TURN_EVENT_QUEUE_CAPACITY);
        let driver = Arc::new(Self {
            runtime: Arc::clone(&runtime),
            events,
            pending_requests: Mutex::new(BTreeMap::new()),
        });
        let handler: Arc<dyn UpstreamEventHandler> = driver.clone();
        runtime.install_handler(&handler);
        (driver, receiver)
    }

    pub fn start_thread(
        &self,
        request: &StartCodexThread,
    ) -> Result<CodexThreadId, CodexTurnError> {
        let mut params = json!({
            "cwd": request.cwd.as_str(),
            "approvalPolicy": request.access.approval_policy(),
            "sandbox": request.access.sandbox(),
            "serviceName": "zeta",
        });
        if let Some(model) = &request.model {
            params["model"] = Value::String(model.clone());
        }
        let response = self
            .runtime
            .request("thread/start", params)
            .map_err(turn_process_error)?;
        parse_thread_id(&response)
    }

    pub fn resume_read_only_thread(
        &self,
        thread_id: &CodexThreadId,
        cwd: &Path,
        model: Option<&str>,
    ) -> Result<CodexThreadId, CodexTurnError> {
        self.resume_thread(thread_id, cwd, model, CodexThreadAccess::ReadOnly)
    }

    pub fn resume_workspace_write_thread(
        &self,
        thread_id: &CodexThreadId,
        cwd: &Path,
        model: Option<&str>,
    ) -> Result<CodexThreadId, CodexTurnError> {
        self.resume_thread(thread_id, cwd, model, CodexThreadAccess::WorkspaceWrite)
    }

    fn resume_thread(
        &self,
        thread_id: &CodexThreadId,
        cwd: &Path,
        model: Option<&str>,
        access: CodexThreadAccess,
    ) -> Result<CodexThreadId, CodexTurnError> {
        if !cwd.is_absolute() {
            return Err(CodexTurnError::invalid_input(
                "Codex thread working directory must be absolute",
            ));
        }
        let cwd = cwd.to_str().ok_or_else(|| {
            CodexTurnError::invalid_input("Codex thread working directory must be valid UTF-8")
        })?;
        if model.is_some_and(|model| model.trim().is_empty()) {
            return Err(CodexTurnError::invalid_input(
                "Codex thread model must not be empty",
            ));
        }
        let response = self
            .runtime
            .request(
                "thread/resume",
                json!({
                    "threadId": thread_id.as_str(),
                    "cwd": cwd,
                    "model": model,
                    "approvalPolicy": access.approval_policy(),
                    "sandbox": access.sandbox(),
                }),
            )
            .map_err(turn_process_error)?;
        parse_thread_id(&response)
    }

    pub fn start_turn(&self, request: &StartCodexTurn) -> Result<CodexTurnId, CodexTurnError> {
        let response = self
            .runtime
            .request(
                "turn/start",
                json!({
                    "threadId": request.thread_id.as_str(),
                    "input": [{
                        "type": "text",
                        "text": request.text.as_str(),
                    }],
                }),
            )
            .map_err(turn_process_error)?;
        parse_turn_id(&response)
    }

    /// Appends text input to the exact active upstream Turn.
    pub fn steer_turn(
        &self,
        thread_id: &CodexThreadId,
        turn_id: &CodexTurnId,
        input: &[String],
    ) -> Result<(), CodexTurnError> {
        if input.is_empty() || input.iter().any(|text| text.trim().is_empty()) {
            return Err(CodexTurnError::invalid_input(
                "Codex Turn steering requires non-empty text input",
            ));
        }
        let response = self
            .runtime
            .request(
                "turn/steer",
                json!({
                    "threadId": thread_id.as_str(),
                    "expectedTurnId": turn_id.as_str(),
                    "input": input
                        .iter()
                        .map(|text| json!({"type": "text", "text": text}))
                        .collect::<Vec<_>>(),
                }),
            )
            .map_err(turn_process_error)?;
        let returned = CodexTurnId::new(required_string(
            &response,
            "/turnId",
            "Turn steer response",
        )?)?;
        if &returned != turn_id {
            return Err(CodexTurnError::incompatible(
                "Turn steer response does not match the expected Turn",
            ));
        }
        Ok(())
    }

    pub fn interrupt(
        &self,
        thread_id: &CodexThreadId,
        turn_id: &CodexTurnId,
    ) -> Result<(), CodexTurnError> {
        self.runtime
            .request(
                "turn/interrupt",
                json!({
                    "threadId": thread_id.as_str(),
                    "turnId": turn_id.as_str(),
                }),
            )
            .map_err(turn_process_error)?;
        Ok(())
    }

    /// Resolve one command or file-change approval exactly once.
    pub fn resolve_approval(
        &self,
        request_id: &CodexServerRequestId,
        decision: CodexApprovalDecision,
    ) -> Result<(), CodexTurnError> {
        self.take_pending_approval(request_id)?;
        self.runtime
            .respond(
                request_id.connection_id,
                request_id.wire_id.clone(),
                json!({ "decision": decision.wire_name() }),
            )
            .map_err(turn_process_error)
    }

    /// Submit answers for one user-input request exactly once.
    pub fn submit_user_input(
        &self,
        request_id: &CodexServerRequestId,
        answers: &CodexUserInputAnswers,
    ) -> Result<(), CodexTurnError> {
        self.take_pending(request_id, PendingServerRequestKind::UserInput)?;
        self.runtime
            .respond(
                request_id.connection_id,
                request_id.wire_id.clone(),
                json!({ "answers": answers.wire_value() }),
            )
            .map_err(turn_process_error)
    }

    /// Rejects a pending upstream request that cannot be represented safely by the caller.
    pub fn reject_server_request(
        &self,
        request_id: &CodexServerRequestId,
    ) -> Result<(), CodexTurnError> {
        self.take_pending_any(request_id)?;
        self.runtime
            .respond_error(
                request_id.connection_id,
                request_id.wire_id.clone(),
                -32001,
                "Zeta cannot safely represent this Codex server request",
            )
            .map_err(turn_process_error)
    }

    fn emit(&self, event: CodexTurnEvent) -> EventHandling {
        if self.events.send(event).is_ok() {
            EventHandling::Handled
        } else {
            EventHandling::Ignored
        }
    }

    fn decode_notification(&self, method: &str, params: &Value) -> EventHandling {
        let event = match method {
            "turn/started" => decode_started(params),
            "item/agentMessage/delta" => decode_item_delta(params, ItemDeltaKind::AgentMessage),
            "item/reasoning/summaryTextDelta" => {
                decode_item_delta(params, ItemDeltaKind::ReasoningSummary)
            }
            "item/reasoning/textDelta" => decode_item_delta(params, ItemDeltaKind::Reasoning),
            "turn/diff/updated" => decode_diff(params),
            "turn/completed" => decode_completed(params),
            _ => return EventHandling::Ignored,
        };
        match event {
            Ok(event) => self.emit(event),
            Err(_) => self.emit(CodexTurnEvent::ProtocolError {
                method: method.into(),
            }),
        }
    }

    fn handle_server_request(
        &self,
        connection_id: UpstreamConnectionId,
        id: &Value,
        method: &str,
        params: &Value,
    ) -> EventHandling {
        let decoded = match decode_server_request(connection_id, id, method, params) {
            Ok(Some(decoded)) => decoded,
            Ok(None) => return EventHandling::Ignored,
            Err(_) => {
                let _ = self.runtime.respond_error(
                    connection_id,
                    id.clone(),
                    -32602,
                    "Codex server request parameters are incompatible with Zeta",
                );
                let _ = self.emit(CodexTurnEvent::ProtocolError {
                    method: method.into(),
                });
                return EventHandling::Handled;
            }
        };
        let inserted = self
            .pending_requests
            .lock()
            .map(|mut pending| {
                if pending.contains_key(&decoded.key) {
                    false
                } else {
                    pending.insert(decoded.key.clone(), decoded.kind);
                    true
                }
            })
            .unwrap_or(false);
        if !inserted {
            let _ = self.runtime.respond_error(
                connection_id,
                id.clone(),
                -32600,
                "Codex server request ID is already pending",
            );
            return EventHandling::Handled;
        }
        if matches!(self.emit(decoded.event), EventHandling::Handled) {
            return EventHandling::Handled;
        }
        if let Ok(mut pending) = self.pending_requests.lock() {
            pending.remove(&decoded.key);
        }
        let _ = self.runtime.respond_error(
            connection_id,
            id.clone(),
            -32000,
            "Zeta cannot receive the Codex server request",
        );
        EventHandling::Handled
    }

    fn take_pending_approval(
        &self,
        request_id: &CodexServerRequestId,
    ) -> Result<(), CodexTurnError> {
        let key = request_id.pending_key();
        let mut pending = self
            .pending_requests
            .lock()
            .map_err(|_| CodexTurnError::conflict("Codex pending-request state was unavailable"))?;
        match pending.get(&key) {
            Some(PendingServerRequestKind::CommandApproval)
            | Some(PendingServerRequestKind::FileChangeApproval) => {
                pending.remove(&key);
                Ok(())
            }
            Some(PendingServerRequestKind::UserInput) => Err(CodexTurnError::conflict(
                "Codex server request expects user-input answers, not an approval decision",
            )),
            None => Err(already_resolved_request()),
        }
    }

    fn take_pending(
        &self,
        request_id: &CodexServerRequestId,
        expected: PendingServerRequestKind,
    ) -> Result<(), CodexTurnError> {
        let key = request_id.pending_key();
        let mut pending = self
            .pending_requests
            .lock()
            .map_err(|_| CodexTurnError::conflict("Codex pending-request state was unavailable"))?;
        match pending.get(&key) {
            Some(actual) if *actual == expected => {
                pending.remove(&key);
                Ok(())
            }
            Some(_) => Err(CodexTurnError::conflict(
                "Codex server request was resolved with the wrong response type",
            )),
            None => Err(already_resolved_request()),
        }
    }

    fn take_pending_any(&self, request_id: &CodexServerRequestId) -> Result<(), CodexTurnError> {
        self.pending_requests
            .lock()
            .map_err(|_| CodexTurnError::conflict("Codex pending-request state was unavailable"))?
            .remove(&request_id.pending_key())
            .map(|_| ())
            .ok_or_else(already_resolved_request)
    }
}

impl UpstreamEventHandler for CodexTurnDriver {
    fn handle_event(
        &self,
        connection_id: UpstreamConnectionId,
        event: &UpstreamEvent,
    ) -> EventHandling {
        match event {
            UpstreamEvent::Notification { method, params } => {
                self.decode_notification(method, params)
            }
            UpstreamEvent::Request { id, method, params } => {
                self.handle_server_request(connection_id, id, method, params)
            }
            UpstreamEvent::ConnectionClosed => self.emit(CodexTurnEvent::ProtocolError {
                method: "runtime/connectionClosed".into(),
            }),
        }
    }
}

fn already_resolved_request() -> CodexTurnError {
    CodexTurnError::conflict("Codex server request is unknown or was already resolved")
}

enum ItemDeltaKind {
    AgentMessage,
    ReasoningSummary,
    Reasoning,
}

fn parse_thread_id(response: &Value) -> Result<CodexThreadId, CodexTurnError> {
    CodexThreadId::new(required_string(response, "/thread/id", "thread response")?)
}

fn parse_turn_id(response: &Value) -> Result<CodexTurnId, CodexTurnError> {
    CodexTurnId::new(required_string(response, "/turn/id", "Turn response")?)
}

fn decode_started(params: &Value) -> Result<CodexTurnEvent, CodexTurnError> {
    Ok(CodexTurnEvent::Started {
        thread_id: CodexThreadId::new(required_string(params, "/threadId", "turn/started")?)?,
        turn_id: CodexTurnId::new(required_string(params, "/turn/id", "turn/started")?)?,
    })
}

fn decode_item_delta(
    params: &Value,
    kind: ItemDeltaKind,
) -> Result<CodexTurnEvent, CodexTurnError> {
    let thread_id = CodexThreadId::new(required_string(params, "/threadId", "item delta")?)?;
    let turn_id = CodexTurnId::new(required_string(params, "/turnId", "item delta")?)?;
    let item_id = required_string(params, "/itemId", "item delta")?.into();
    let delta = required_string(params, "/delta", "item delta")?.into();
    Ok(match kind {
        ItemDeltaKind::AgentMessage => CodexTurnEvent::AgentMessageDelta {
            thread_id,
            turn_id,
            item_id,
            delta,
        },
        ItemDeltaKind::ReasoningSummary => CodexTurnEvent::ReasoningSummaryDelta {
            thread_id,
            turn_id,
            item_id,
            delta,
        },
        ItemDeltaKind::Reasoning => CodexTurnEvent::ReasoningDelta {
            thread_id,
            turn_id,
            item_id,
            delta,
        },
    })
}

fn decode_diff(params: &Value) -> Result<CodexTurnEvent, CodexTurnError> {
    Ok(CodexTurnEvent::DiffUpdated {
        thread_id: CodexThreadId::new(required_string(params, "/threadId", "turn/diff/updated")?)?,
        turn_id: CodexTurnId::new(required_string(params, "/turnId", "turn/diff/updated")?)?,
        diff: required_string(params, "/diff", "turn/diff/updated")?.into(),
    })
}

fn decode_completed(params: &Value) -> Result<CodexTurnEvent, CodexTurnError> {
    let status = match required_string(params, "/turn/status", "turn/completed")? {
        "completed" => CodexTurnStatus::Completed,
        "interrupted" => CodexTurnStatus::Interrupted,
        "failed" => CodexTurnStatus::Failed,
        _ => {
            return Err(CodexTurnError::incompatible(
                "Codex turn/completed contains an invalid status",
            ));
        }
    };
    Ok(CodexTurnEvent::Completed {
        thread_id: CodexThreadId::new(required_string(params, "/threadId", "turn/completed")?)?,
        turn_id: CodexTurnId::new(required_string(params, "/turn/id", "turn/completed")?)?,
        status,
    })
}

fn required_string<'a>(
    value: &'a Value,
    pointer: &str,
    _context: &'static str,
) -> Result<&'a str, CodexTurnError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            CodexTurnError::incompatible("Codex App Server returned an incompatible Turn response")
        })
}

fn turn_process_error(error: ProcessError) -> CodexTurnError {
    let kind = match error.kind {
        ProcessErrorKind::Unavailable => CodexTurnErrorKind::Unavailable,
        ProcessErrorKind::Unsupported => CodexTurnErrorKind::Unsupported,
        ProcessErrorKind::Rejected => CodexTurnErrorKind::Rejected,
    };
    CodexTurnError {
        kind,
        message: error.message,
    }
}
