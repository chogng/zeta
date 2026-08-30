use crate::SessionId;
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
    pub threads: Vec<SessionThread>,
}

/// One Thread grouped into a Session tree by `session_id`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThread {
    pub thread_id: ThreadId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub parent_thread_id: Option<ThreadId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub forked_from_id: Option<ThreadId>,
    pub status: ThreadStatus,
}
