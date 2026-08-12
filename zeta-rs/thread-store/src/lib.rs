//! Storage-neutral durable Thread history boundary.

mod error;
mod store;

pub use error::ThreadStoreError;
pub use store::AppendBatchResult;
pub use store::ThreadEventBatch;
pub use store::ThreadHistoryPage;
pub use store::ThreadHistoryQuery;
pub use store::ThreadStore;
pub use store::history_page_from_events;
pub use store::validate_append_batch;
pub use store::validate_history_query;
