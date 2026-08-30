use crate::ThreadStoreError;
use serde::Deserialize;
use serde::Serialize;
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::StoredEvent;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;

/// Lightweight durable facts used to list Sessions without replaying Thread history.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThreadCatalogRecord {
    pub session_id: SessionId,
    pub thread: SessionThread,
    pub sequence: u64,
    pub manager: SessionManagerInfo,
    pub archived_at_unix_ms: Option<u64>,
    pub stopped: bool,
    pub requires_startup_recovery: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadEventBatch {
    pub batch_id: String,
    pub thread_id: ThreadId,
    pub expected_sequence: u64,
    pub events: Vec<StoredEvent>,
    pub catalog: ThreadCatalogRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendBatchResult {
    pub batch_id: String,
    pub committed_sequence: u64,
    pub event_count: usize,
}

/// Loads and atomically extends the authoritative event history for one Thread.
///
/// Implementations must reject stale `expected_sequence` values. `append_batch` commits every
/// event or none, makes the complete batch durable before returning success, and excludes
/// uncommitted tail batches from subsequent `load` results.
pub trait ThreadStore: Send + Sync {
    fn list_thread_ids(&self) -> Result<Vec<ThreadId>, ThreadStoreError>;

    fn list_catalog(&self) -> Result<Vec<ThreadCatalogRecord>, ThreadStoreError>;

    /// Installs a missing catalog row while upgrading an older event store.
    fn backfill_catalog(&self, record: &ThreadCatalogRecord) -> Result<(), ThreadStoreError>;

    fn load(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError>;

    fn append_batch(&self, batch: &ThreadEventBatch)
    -> Result<AppendBatchResult, ThreadStoreError>;
}

pub fn validate_append_batch(
    batch: &ThreadEventBatch,
    actual_sequence: u64,
) -> Result<AppendBatchResult, ThreadStoreError> {
    if batch.expected_sequence != actual_sequence {
        return Err(ThreadStoreError::SequenceConflict {
            expected: batch.expected_sequence,
            actual: actual_sequence,
        });
    }
    if batch.batch_id.trim().is_empty() {
        return Err(ThreadStoreError::InvalidBatch(
            "batch ID must not be empty".into(),
        ));
    }
    if batch.events.is_empty() {
        return Err(ThreadStoreError::InvalidBatch(
            "batch must contain at least one event".into(),
        ));
    }
    for (index, event) in batch.events.iter().enumerate() {
        let sequence = batch.expected_sequence + index as u64 + 1;
        if event.schema_version != CURRENT_STORED_EVENT_SCHEMA_VERSION {
            return Err(ThreadStoreError::InvalidBatch(
                "new events must use the current schema version".into(),
            ));
        }
        if event.thread_id != batch.thread_id
            || event.event.thread_id() != &batch.thread_id
            || event.sequence != sequence
        {
            return Err(ThreadStoreError::InvalidBatch(
                "event Thread identity or sequence does not match its batch".into(),
            ));
        }
        if batch.events[..index]
            .iter()
            .any(|existing| existing.event_id == event.event_id)
        {
            return Err(ThreadStoreError::InvalidBatch(
                "event IDs must be unique within a batch".into(),
            ));
        }
    }
    if batch.catalog.thread.thread_id != batch.thread_id
        || batch.catalog.sequence != batch.expected_sequence + batch.events.len() as u64
    {
        return Err(ThreadStoreError::InvalidBatch(
            "catalog Thread identity or sequence does not match its batch".into(),
        ));
    }
    Ok(AppendBatchResult {
        batch_id: batch.batch_id.clone(),
        committed_sequence: batch.expected_sequence + batch.events.len() as u64,
        event_count: batch.events.len(),
    })
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
