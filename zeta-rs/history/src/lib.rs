//! Model-history and persisted Thread-record domain types.
//!
//! This crate defines the data that survives process restarts. It deliberately owns no database,
//! filesystem, pagination, append transaction, or reducer implementation.

mod record;

pub use record::CURRENT_STORED_EVENT_SCHEMA_VERSION;
pub use record::EventId;
pub use record::MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION;
pub use record::StoredEvent;
pub use record::ThreadCommandReceipt;
pub use record::Timestamp;
pub use record::supports_stored_event_schema_version;

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
