use crate::ApprovalMode;
use crate::ModelRef;
use crate::SessionId;
use crate::SessionStatus;
use crate::SessionThreadStatus;
use crate::ThreadId;
use crate::ThreadOrigin;
use crate::WorkspaceBinding;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Canonical user-visible work container and its Thread topology.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub workspace: Option<WorkspaceBinding>,
    #[serde(default)]
    pub next_approval_mode: ApprovalMode,
    /// Canonical conversation route resumed by every product client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub current_thread_id: Option<ThreadId>,
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
