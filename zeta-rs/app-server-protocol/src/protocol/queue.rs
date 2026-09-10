use crate::protocol::turn::InputItem;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CommandId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ToolMode;

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueEnqueueParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub input: Vec<InputItem>,
    pub tool_mode: ToolMode,
    pub approval_mode: ApprovalMode,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueListParams {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueCancelParams {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub command_id: CommandId,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct QueueListResult {
    pub messages: Vec<queue::QueuedMessage>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueEditParams {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: i64,
    pub action: QueueEditAction,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QueueEditAction {
    Pause,
    Replace {
        input: Vec<InputItem>,
    },
    Move {
        direction: queue::QueueMove,
    },
    Send {
        turn_id: Option<zeta_protocol::TurnId>,
    },
}
