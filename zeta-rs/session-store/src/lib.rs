//! Storage-neutral durable product Session history boundary.

mod error;
mod record;
mod store;

pub use error::SessionStoreError;
pub use record::{
    CURRENT_SESSION_EVENT_SCHEMA_VERSION, MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION,
    SessionCommandReceipt, SessionEventId, SessionTimestamp, StoredSessionEvent,
    supports_session_event_schema_version,
};
pub use store::{
    AppendSessionBatchResult, SessionEventBatch, SessionStore, validate_session_append_batch,
    validate_session_history,
};
