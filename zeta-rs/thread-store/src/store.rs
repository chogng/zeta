use crate::ThreadStoreError;
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::StoredEvent;
use zeta_protocol::ThreadId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadEventBatch {
    pub batch_id: String,
    pub thread_id: ThreadId,
    pub expected_sequence: u64,
    pub events: Vec<StoredEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendBatchResult {
    pub batch_id: String,
    pub committed_sequence: u64,
    pub event_count: usize,
}

/// Selects one page of an authoritative Thread event stream.
///
/// `before_sequence` is an exclusive durable sequence cursor. `None` selects the newest
/// events; a returned `next_before_sequence` can be passed to the next query to walk toward
/// the beginning of the stream. Implementations return events in ascending sequence order so
/// callers can append a page without reversing the durable ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThreadHistoryQuery {
    pub before_sequence: Option<u64>,
    pub limit: usize,
}

/// A bounded page from one Thread's durable event stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadHistoryPage {
    pub events: Vec<StoredEvent>,
    pub next_before_sequence: Option<u64>,
}

/// Loads and atomically extends the authoritative event history for one Thread.
///
/// Implementations must reject stale `expected_sequence` values. `append_batch` commits every
/// event or none, makes the complete batch durable before returning success, and excludes
/// uncommitted tail batches from subsequent `load` results.
pub trait ThreadStore: Send + Sync {
    fn list_thread_ids(&self) -> Result<Vec<ThreadId>, ThreadStoreError>;

    fn load(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError>;

    /// Loads a bounded durable event page without changing the authoritative event stream.
    ///
    /// The default implementation preserves compatibility for non-indexed stores. Production
    /// stores should override it with a bounded query so long histories do not require loading
    /// the complete event log into memory for one history page.
    fn load_history_page(
        &self,
        thread_id: &ThreadId,
        query: ThreadHistoryQuery,
    ) -> Result<ThreadHistoryPage, ThreadStoreError> {
        validate_history_query(query)?;
        history_page_from_events(&self.load(thread_id)?, query)
    }

    fn append_batch(&self, batch: &ThreadEventBatch)
    -> Result<AppendBatchResult, ThreadStoreError>;
}

pub fn validate_history_query(query: ThreadHistoryQuery) -> Result<(), ThreadStoreError> {
    if query.limit == 0 {
        return Err(ThreadStoreError::InvalidQuery(
            "history page limit must be positive".into(),
        ));
    }
    if query.before_sequence == Some(0) {
        return Err(ThreadStoreError::InvalidQuery(
            "history cursor must be greater than zero".into(),
        ));
    }
    Ok(())
}

pub fn history_page_from_events(
    events: &[StoredEvent],
    query: ThreadHistoryQuery,
) -> Result<ThreadHistoryPage, ThreadStoreError> {
    validate_history_query(query)?;
    let eligible_end = query.before_sequence.map_or(events.len(), |end| {
        events.partition_point(|event| event.sequence < end)
    });
    let start = eligible_end.saturating_sub(query.limit);
    let page = events[start..eligible_end].to_vec();
    let next_before_sequence = (start > 0)
        .then(|| page.first().map(|event| event.sequence))
        .flatten();
    Ok(ThreadHistoryPage {
        events: page,
        next_before_sequence,
    })
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
    Ok(AppendBatchResult {
        batch_id: batch.batch_id.clone(),
        committed_sequence: batch.expected_sequence + batch.events.len() as u64,
        event_count: batch.events.len(),
    })
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
