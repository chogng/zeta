use crate::{ModelRef, SessionId, SessionStatus, SessionThreadStatus, ThreadId, ThreadOrigin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Canonical product-level container for a task and its Thread topology.
///
/// A Session owns Thread membership and lineage, but not the child Threads' Turn or Item history.
/// Each Thread remains an independent ordering, persistence, and execution boundary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: SessionId,
    pub title: String,
    pub status: SessionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub model: Option<ModelRef>,
    #[ts(type = "number")]
    pub sequence: u64,
    pub threads: Vec<SessionThread>,
}

/// A Session-owned reference to one Thread, its origin, and membership lifecycle.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThread {
    pub thread_id: ThreadId,
    pub origin: ThreadOrigin,
    pub status: SessionThreadStatus,
}
