use serde::{Deserialize, Serialize};
use zeta_protocol::{CommandId, ThreadCommand, ThreadEvent, ThreadId};

pub const CURRENT_STORED_EVENT_SCHEMA_VERSION: u32 = 2;
pub const MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventId(pub String);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Timestamp(pub u128);

/// The exact typed command durably accepted by a Thread stream.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadCommandReceipt {
    pub command_id: CommandId,
    pub command: ThreadCommand,
}

/// Storage-owned envelope for an event persisted in a thread rollout.
///
/// Only durable `zeta_protocol::ThreadEvent` values can enter this envelope. Implementations of
/// `ThreadStore` add ordering, timestamps, schema versions, and idempotency metadata at the
/// persistence boundary; live `ThreadUpdate` values are never stored here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredEvent {
    pub schema_version: u32,
    pub event_id: EventId,
    pub sequence: u64,
    pub thread_id: ThreadId,
    pub recorded_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<ThreadCommandReceipt>,
    pub event: ThreadEvent,
}

impl StoredEvent {
    pub fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }
}
