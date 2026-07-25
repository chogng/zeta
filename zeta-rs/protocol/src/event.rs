use crate::ThreadId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventId(pub String);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Timestamp(pub u128);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentEvent {
    pub event_id: EventId,
    pub sequence: u64,
    pub thread_id: ThreadId,
    pub kind: String,
    /// Canonical JSON payload for the event kind; the stable history never stores secrets here.
    pub payload: String,
    pub occurred_at: Timestamp,
}
