use serde::Deserialize;
use serde::Serialize;
use zeta_protocol::CommandId;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;

/// Schema version written for newly persisted Thread history records.
pub const CURRENT_STORED_EVENT_SCHEMA_VERSION: u32 = 7;

/// Oldest Thread history record schema accepted during recovery.
pub const MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION: u32 = 1;

/// Returns whether a persisted Thread history record can be replayed by this build.
pub const fn supports_stored_event_schema_version(schema_version: u32) -> bool {
    schema_version >= MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION
        && schema_version <= CURRENT_STORED_EVENT_SCHEMA_VERSION
}

/// Stable identity of one persisted Thread history record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventId(pub String);

/// Unix timestamp in milliseconds attached to a persisted Thread history record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Timestamp(pub u128);

/// The exact typed command durably accepted by a Thread stream.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadCommandReceipt {
    pub command_id: CommandId,
    pub command: ThreadCommand,
}

/// Canonical persisted envelope for one durable Thread fact.
///
/// Only durable [`ThreadEvent`] values can enter this envelope. Core constructs the record when a
/// command is accepted; a Thread Store validates and persists the exact value. Live updates, token
/// deltas, storage transactions, and query cursors do not belong to this data contract.
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
