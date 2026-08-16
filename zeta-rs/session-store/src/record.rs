use serde::{Deserialize, Serialize};
use zeta_protocol::{CommandId, SessionCommand, SessionEvent, SessionId};

pub const CURRENT_SESSION_EVENT_SCHEMA_VERSION: u32 = 3;
pub const MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION: u32 = 1;

pub const fn supports_session_event_schema_version(schema_version: u32) -> bool {
    schema_version >= MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION
        && schema_version <= CURRENT_SESSION_EVENT_SCHEMA_VERSION
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SessionEventId(pub String);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SessionTimestamp(pub u128);

/// The exact typed command durably accepted by a Session stream.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCommandReceipt {
    pub command_id: CommandId,
    pub command: SessionCommand,
}

/// One durable Session fact and optional command receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSessionEvent {
    pub schema_version: u32,
    pub event_id: SessionEventId,
    pub sequence: u64,
    pub session_id: SessionId,
    pub recorded_at: SessionTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<SessionCommandReceipt>,
    pub event: SessionEvent,
}
