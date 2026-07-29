use crate::agent::{ReplyAgentRequest, StartAgentRequest};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fmt;
use std::time::Duration;

pub(crate) const MCP_PROTOCOL_VERSION: &str = "2025-11-25";
pub(crate) const TOOL_START: &str = "zeta";
pub(crate) const TOOL_REPLY: &str = "zeta-reply";
const MAX_INVOCATION_ID_BYTES: usize = 128;
const MAX_PROMPT_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
#[serde(untagged)]
pub(crate) enum JsonRpcId {
    Number(i64),
    String(String),
}

impl fmt::Display for JsonRpcId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => write!(formatter, "{value}"),
            Self::String(value) => formatter.write_str(value),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct IncomingMessage {
    pub(crate) jsonrpc: String,
    #[serde(default)]
    pub(crate) id: Option<JsonRpcId>,
    pub(crate) method: String,
    #[serde(default)]
    pub(crate) params: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitializeParams {
    pub(crate) protocol_version: String,
    pub(crate) capabilities: Value,
    pub(crate) client_info: Implementation,
}

#[derive(Deserialize)]
pub(crate) struct Implementation {
    pub(crate) name: String,
    pub(crate) version: String,
}

#[derive(Deserialize)]
pub(crate) struct CallToolParams {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) arguments: Value,
    #[serde(default, rename = "_meta")]
    meta: RequestMeta,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestMeta {
    progress_token: Option<Value>,
}

impl CallToolParams {
    pub(crate) fn progress_token(&self) -> Result<Option<Value>, String> {
        let Some(token) = self.meta.progress_token.clone() else {
            return Ok(None);
        };
        match &token {
            Value::String(_) => Ok(Some(token)),
            Value::Number(number) if number.is_i64() || number.is_u64() => Ok(Some(token)),
            _ => Err("_meta.progressToken must be a string or integer".into()),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartToolArguments {
    invocation_id: String,
    prompt: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReplyToolArguments {
    invocation_id: String,
    thread_id: String,
    prompt: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelledParams {
    pub(crate) request_id: JsonRpcId,
}

pub(crate) fn decode_start(arguments: Value) -> Result<StartAgentRequest, String> {
    let arguments: StartToolArguments =
        serde_json::from_value(arguments).map_err(|error| error.to_string())?;
    validate_invocation_id(&arguments.invocation_id)?;
    validate_prompt(&arguments.prompt)?;
    Ok(StartAgentRequest {
        invocation_id: arguments.invocation_id,
        prompt: arguments.prompt,
        timeout: arguments.timeout_ms.map(Duration::from_millis),
    })
}

pub(crate) fn decode_reply(arguments: Value) -> Result<ReplyAgentRequest, String> {
    let arguments: ReplyToolArguments =
        serde_json::from_value(arguments).map_err(|error| error.to_string())?;
    validate_invocation_id(&arguments.invocation_id)?;
    if arguments.thread_id.trim().is_empty() {
        return Err("threadId must not be empty".into());
    }
    validate_prompt(&arguments.prompt)?;
    Ok(ReplyAgentRequest {
        invocation_id: arguments.invocation_id,
        thread_id: arguments.thread_id,
        prompt: arguments.prompt,
        timeout: arguments.timeout_ms.map(Duration::from_millis),
    })
}

pub(crate) fn initialize_result() -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "zeta-mcp-server",
            "title": "Zeta",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Use `zeta` to start an independent Zeta Agent task and `zeta-reply` to continue a Thread created by this connection."
    })
}

pub(crate) fn tools_result() -> Value {
    json!({
        "tools": [
            {
                "name": TOOL_START,
                "title": "Zeta",
                "description": "Start an independent Zeta Agent task in the server's authorized Workspace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "invocationId": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": MAX_INVOCATION_ID_BYTES,
                            "description": "Stable caller-generated idempotency identity."
                        },
                        "prompt": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": MAX_PROMPT_BYTES
                        },
                        "timeoutMs": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Optional Turn deadline in milliseconds, bounded by the server."
                        }
                    },
                    "required": ["invocationId", "prompt"],
                    "additionalProperties": false
                },
                "outputSchema": agent_output_schema()
            },
            {
                "name": TOOL_REPLY,
                "title": "Zeta Reply",
                "description": "Continue a Zeta Agent Thread created by this MCP connection.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "invocationId": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": MAX_INVOCATION_ID_BYTES,
                            "description": "Stable caller-generated idempotency identity for this follow-up."
                        },
                        "threadId": {
                            "type": "string",
                            "minLength": 1
                        },
                        "prompt": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": MAX_PROMPT_BYTES
                        },
                        "timeoutMs": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Optional Turn deadline in milliseconds, bounded by the server."
                        }
                    },
                    "required": ["invocationId", "threadId", "prompt"],
                    "additionalProperties": false
                },
                "outputSchema": agent_output_schema()
            }
        ]
    })
}

fn agent_output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "invocationId": {"type": "string"},
            "sessionId": {"type": "string"},
            "threadId": {"type": "string"},
            "turnId": {"type": "string"},
            "status": {
                "type": "string",
                "enum": [
                    "completed",
                    "waitingForApproval",
                    "waitingForUserInput",
                    "waitingForCapability",
                    "failed",
                    "interrupted",
                    "outcomeUnknown"
                ]
            },
            "content": {"type": "string"}
        },
        "required": [
            "invocationId",
            "sessionId",
            "threadId",
            "turnId",
            "status",
            "content"
        ],
        "additionalProperties": false
    })
}

fn validate_invocation_id(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("invocationId must not be empty".into());
    }
    if value.len() > MAX_INVOCATION_ID_BYTES {
        return Err(format!(
            "invocationId exceeds {MAX_INVOCATION_ID_BYTES} bytes"
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("invocationId may contain only ASCII letters, digits, '.', '_' and '-'".into());
    }
    Ok(())
}

fn validate_prompt(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err("prompt must not be empty".into());
    }
    if value.len() > MAX_PROMPT_BYTES {
        return Err(format!("prompt exceeds {MAX_PROMPT_BYTES} bytes"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
