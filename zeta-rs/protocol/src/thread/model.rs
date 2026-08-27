use crate::{ModelUsageSummary, SessionId, ThreadGoal, ThreadId, ThreadStatus, Turn};
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
    #[serde(default)]
    pub usage: ModelUsageSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub goal: Option<ThreadGoal>,
    pub turns: Vec<Turn>,
}
