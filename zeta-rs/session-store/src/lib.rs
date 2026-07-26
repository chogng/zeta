//! Storage-neutral durable product Session history boundary.

mod error;
mod record;
mod store;

pub use error::SessionStoreError;
pub use record::{
    CURRENT_SESSION_EVENT_SCHEMA_VERSION, SessionCommandReceipt, SessionEventId, SessionTimestamp,
    StoredSessionEvent,
};
pub use store::{
    AppendSessionBatchResult, SessionEventBatch, SessionStore, validate_session_append_batch,
};
