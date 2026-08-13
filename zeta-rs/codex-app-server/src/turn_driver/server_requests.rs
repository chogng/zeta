use super::CodexThreadId;
use super::CodexTurnError;
use super::CodexTurnEvent;
use super::CodexTurnId;
use crate::runtime::UpstreamConnectionId;
use serde_json::Value;
use std::collections::BTreeMap;

/// Opaque identity for one upstream request that must receive exactly one response.
#[derive(Clone, Debug)]
pub struct CodexServerRequestId {
    pub(super) connection_id: UpstreamConnectionId,
    pub(super) key: String,
    pub(super) wire_id: Value,
}

impl PartialEq for CodexServerRequestId {
    fn eq(&self, other: &Self) -> bool {
        self.connection_id == other.connection_id && self.key == other.key
    }
}

impl Eq for CodexServerRequestId {}

/// Decisions shared by command-execution and file-change approval prompts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexApprovalDecision {
    Accept,
    AcceptForSession,
    Decline,
    Cancel,
}

impl CodexApprovalDecision {
    pub(super) fn wire_name(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::AcceptForSession => "acceptForSession",
            Self::Decline => "decline",
            Self::Cancel => "cancel",
        }
    }

    fn from_wire_name(value: &str) -> Option<Self> {
        match value {
            "accept" => Some(Self::Accept),
            "acceptForSession" => Some(Self::AcceptForSession),
            "decline" => Some(Self::Decline),
            "cancel" => Some(Self::Cancel),
            _ => None,
        }
    }
}

/// A command that Codex wants permission to execute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexCommandApprovalRequest {
    pub request_id: CodexServerRequestId,
    pub thread_id: CodexThreadId,
    pub turn_id: CodexTurnId,
    pub item_id: String,
    pub started_at_ms: i64,
    pub command: String,
    pub cwd: Option<String>,
    pub reason: Option<String>,
    /// Empty when the upstream server did not advertise a restricted choice set.
    pub available_decisions: Vec<CodexApprovalDecision>,
}

/// A file change that Codex wants permission to apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexFileChangeApprovalRequest {
    pub request_id: CodexServerRequestId,
    pub thread_id: CodexThreadId,
    pub turn_id: CodexTurnId,
    pub item_id: String,
    pub started_at_ms: i64,
    pub reason: Option<String>,
    pub grant_root: Option<String>,
}

/// One selectable answer shown for a Codex user-input question.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexUserInputOption {
    pub label: String,
    pub description: String,
}

/// One question in an upstream Codex user-input request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexUserInputQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub allows_other: bool,
    pub is_secret: bool,
    /// Empty for a free-form question.
    pub options: Vec<CodexUserInputOption>,
}

/// A set of questions that must be answered before Codex can continue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexUserInputRequest {
    pub request_id: CodexServerRequestId,
    pub thread_id: CodexThreadId,
    pub turn_id: CodexTurnId,
    pub item_id: String,
    pub questions: Vec<CodexUserInputQuestion>,
    pub is_blocking: bool,
}

/// Answers keyed by the stable question IDs from [`CodexUserInputRequest`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodexUserInputAnswers {
    answers: BTreeMap<String, Vec<String>>,
}

impl CodexUserInputAnswers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn answer(
        mut self,
        question_id: impl Into<String>,
        answers: Vec<String>,
    ) -> Result<Self, CodexTurnError> {
        let question_id = question_id.into();
        if question_id.trim().is_empty() {
            return Err(CodexTurnError::invalid_input(
                "Codex user-input question ID must not be empty",
            ));
        }
        self.answers.insert(question_id, answers);
        Ok(self)
    }

    pub(super) fn wire_value(&self) -> Value {
        Value::Object(
            self.answers
                .iter()
                .map(|(question_id, answers)| {
                    (
                        question_id.clone(),
                        serde_json::json!({ "answers": answers }),
                    )
                })
                .collect(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PendingServerRequestKind {
    CommandApproval,
    FileChangeApproval,
    UserInput,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct PendingServerRequestKey {
    connection_id: UpstreamConnectionId,
    wire_key: String,
}

pub(super) struct DecodedServerRequest {
    pub key: PendingServerRequestKey,
    pub kind: PendingServerRequestKind,
    pub event: CodexTurnEvent,
}

impl CodexServerRequestId {
    pub(super) fn from_wire(
        connection_id: UpstreamConnectionId,
        wire_id: &Value,
    ) -> Result<Self, CodexTurnError> {
        let key = match wire_id {
            Value::String(value) => format!("string:{value}"),
            Value::Number(value) => format!("number:{value}"),
            _ => {
                return Err(CodexTurnError::incompatible(
                    "Codex server request contains an invalid JSON-RPC ID",
                ));
            }
        };
        Ok(Self {
            connection_id,
            key,
            wire_id: wire_id.clone(),
        })
    }

    pub(super) fn pending_key(&self) -> PendingServerRequestKey {
        PendingServerRequestKey {
            connection_id: self.connection_id,
            wire_key: self.key.clone(),
        }
    }
}

pub(super) fn decode_server_request(
    connection_id: UpstreamConnectionId,
    wire_id: &Value,
    method: &str,
    params: &Value,
) -> Result<Option<DecodedServerRequest>, CodexTurnError> {
    let request_id = CodexServerRequestId::from_wire(connection_id, wire_id)?;
    let (kind, event) = match method {
        "item/commandExecution/requestApproval" => (
            PendingServerRequestKind::CommandApproval,
            CodexTurnEvent::CommandApprovalRequested(decode_command_approval(
                request_id.clone(),
                params,
            )?),
        ),
        "item/fileChange/requestApproval" => (
            PendingServerRequestKind::FileChangeApproval,
            CodexTurnEvent::FileChangeApprovalRequested(decode_file_change_approval(
                request_id.clone(),
                params,
            )?),
        ),
        "item/tool/requestUserInput" => (
            PendingServerRequestKind::UserInput,
            CodexTurnEvent::UserInputRequested(decode_user_input(request_id.clone(), params)?),
        ),
        _ => return Ok(None),
    };
    Ok(Some(DecodedServerRequest {
        key: request_id.pending_key(),
        kind,
        event,
    }))
}

fn decode_command_approval(
    request_id: CodexServerRequestId,
    params: &Value,
) -> Result<CodexCommandApprovalRequest, CodexTurnError> {
    Ok(CodexCommandApprovalRequest {
        request_id,
        thread_id: thread_id(params)?,
        turn_id: turn_id(params)?,
        item_id: required_string(params, "/itemId")?.into(),
        started_at_ms: required_i64(params, "/startedAtMs")?,
        command: required_string(params, "/command")?.into(),
        cwd: optional_string(params, "/cwd")?,
        reason: optional_string(params, "/reason")?,
        available_decisions: available_decisions(params)?,
    })
}

fn decode_file_change_approval(
    request_id: CodexServerRequestId,
    params: &Value,
) -> Result<CodexFileChangeApprovalRequest, CodexTurnError> {
    Ok(CodexFileChangeApprovalRequest {
        request_id,
        thread_id: thread_id(params)?,
        turn_id: turn_id(params)?,
        item_id: required_string(params, "/itemId")?.into(),
        started_at_ms: required_i64(params, "/startedAtMs")?,
        reason: optional_string(params, "/reason")?,
        grant_root: optional_string(params, "/grantRoot")?,
    })
}

fn decode_user_input(
    request_id: CodexServerRequestId,
    params: &Value,
) -> Result<CodexUserInputRequest, CodexTurnError> {
    let questions = params
        .pointer("/questions")
        .and_then(Value::as_array)
        .ok_or_else(invalid_server_request)?
        .iter()
        .map(decode_question)
        .collect::<Result<Vec<_>, _>>()?;
    if questions.is_empty() {
        return Err(invalid_server_request());
    }
    Ok(CodexUserInputRequest {
        request_id,
        thread_id: thread_id(params)?,
        turn_id: turn_id(params)?,
        item_id: required_string(params, "/itemId")?.into(),
        questions,
        is_blocking: optional_bool(params, "/isBlocking")?.unwrap_or(true),
    })
}

fn decode_question(value: &Value) -> Result<CodexUserInputQuestion, CodexTurnError> {
    let options = match value.pointer("/options") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(options)) => options
            .iter()
            .map(|option| {
                Ok(CodexUserInputOption {
                    label: required_string(option, "/label")?.into(),
                    description: required_string(option, "/description")?.into(),
                })
            })
            .collect::<Result<Vec<_>, CodexTurnError>>()?,
        Some(_) => return Err(invalid_server_request()),
    };
    Ok(CodexUserInputQuestion {
        id: required_string(value, "/id")?.into(),
        header: required_string(value, "/header")?.into(),
        question: required_string(value, "/question")?.into(),
        allows_other: optional_bool(value, "/isOther")?.unwrap_or(false),
        is_secret: optional_bool(value, "/isSecret")?.unwrap_or(false),
        options,
    })
}

fn available_decisions(params: &Value) -> Result<Vec<CodexApprovalDecision>, CodexTurnError> {
    match params.pointer("/availableDecisions") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(decisions)) => Ok(decisions
            .iter()
            .filter_map(Value::as_str)
            .filter_map(CodexApprovalDecision::from_wire_name)
            .collect()),
        Some(_) => Err(invalid_server_request()),
    }
}

fn thread_id(params: &Value) -> Result<CodexThreadId, CodexTurnError> {
    CodexThreadId::new(required_string(params, "/threadId")?)
}

fn turn_id(params: &Value) -> Result<CodexTurnId, CodexTurnError> {
    CodexTurnId::new(required_string(params, "/turnId")?)
}

fn required_string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, CodexTurnError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(invalid_server_request)
}

fn required_i64(value: &Value, pointer: &str) -> Result<i64, CodexTurnError> {
    value
        .pointer(pointer)
        .and_then(Value::as_i64)
        .ok_or_else(invalid_server_request)
}

fn optional_string(value: &Value, pointer: &str) -> Result<Option<String>, CodexTurnError> {
    match value.pointer(pointer) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(invalid_server_request()),
    }
}

fn optional_bool(value: &Value, pointer: &str) -> Result<Option<bool>, CodexTurnError> {
    match value.pointer(pointer) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(invalid_server_request()),
    }
}

fn invalid_server_request() -> CodexTurnError {
    CodexTurnError::incompatible("Codex App Server sent an incompatible server request")
}
