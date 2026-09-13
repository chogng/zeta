use crate::ModelUsageSummary;
use crate::SessionId;
use crate::SessionManagerInfo;
use crate::SessionStatus;
use crate::ThreadId;
use crate::ThreadStatus;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// A read-only view of Threads grouped by their shared `session_id`.
///
/// Session has no independent event stream, sequence, or mutable runtime state. Thread remains the
/// persistence, ordering, and execution boundary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: SessionId,
    pub title: String,
    pub status: SessionStatus,
    #[serde(default)]
    pub manager: SessionManagerInfo,
    pub threads: Vec<SessionThread>,
}

/// One Thread grouped into a Session tree by `session_id`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThread {
    pub thread_id: ThreadId,
    pub title: String,
    #[ts(type = "number")]
    pub created_at_unix_ms: u64,
    /// Sum of durations for terminal Turns that started in this Thread.
    #[serde(default)]
    #[ts(type = "number")]
    pub completed_turn_duration_ms: u64,
    /// Start time of the current non-terminal Turn, when it has begun running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number")]
    pub active_turn_started_at_unix_ms: Option<u64>,
    /// Provider-reported token usage accumulated by this execution branch.
    #[serde(default)]
    pub usage: ModelUsageSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub parent_thread_id: Option<ThreadId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub forked_from_id: Option<ThreadId>,
    pub status: ThreadStatus,
}
