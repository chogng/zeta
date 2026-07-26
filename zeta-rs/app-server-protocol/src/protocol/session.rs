use crate::protocol::common::{CommandId, SessionId, ThreadId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeta_protocol::{Session, SessionUpdateEnvelope};

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateParams {
    pub command_id: CommandId,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionReadParams {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionSubscribeParams {
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub after_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionUnsubscribeParams {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionCommandParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadCreateParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadForkParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub parent_thread_id: ThreadId,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadArchiveParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub thread_id: ThreadId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionResult {
    pub session: Session,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResult {
    pub sessions: Vec<Session>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionSubscribeResult {
    pub session: Session,
    pub updates: Vec<SessionUpdateEnvelope>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadResult {
    pub session: Session,
    pub thread_id: ThreadId,
}
