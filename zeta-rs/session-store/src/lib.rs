//! Storage-neutral durable product Session history boundary.

mod error;
mod record;
mod store;

pub use error::SessionStoreError;
pub use record::{
    supports_session_event_schema_version, SessionCommandReceipt, SessionEventId, SessionTimestamp,
    StoredSessionEvent, CURRENT_SESSION_EVENT_SCHEMA_VERSION,
    MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION,
};
pub use store::{
    validate_session_append_batch, validate_session_history, AppendSessionBatchResult,
    SessionEventBatch, SessionStore,
};
