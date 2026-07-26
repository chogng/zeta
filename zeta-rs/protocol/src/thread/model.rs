use crate::{SessionId, ThreadId, ThreadStatus, Turn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Canonical readable state for one independently ordered Agent execution branch.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub title: String,
    pub status: ThreadStatus,
    #[ts(type = "number")]
    pub sequence: u64,
    pub turns: Vec<Turn>,
}
