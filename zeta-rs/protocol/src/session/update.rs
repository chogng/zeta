use crate::SessionEvent;
use crate::SessionId;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionUpdate {
    Committed { event: SessionEvent },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdateEnvelope {
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub durable_sequence: u64,
    pub update: SessionUpdate,
}
