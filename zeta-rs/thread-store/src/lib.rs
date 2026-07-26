//! Storage-neutral durable Thread history boundary.

mod error;
mod record;
mod store;

pub use error::ThreadStoreError;
pub use record::CURRENT_STORED_EVENT_SCHEMA_VERSION;
pub use record::EventId;
pub use record::StoredEvent;
pub use record::ThreadCommandReceipt;
pub use record::Timestamp;
pub use store::AppendBatchResult;
pub use store::ThreadEventBatch;
pub use store::ThreadStore;
pub use store::validate_append_batch;
