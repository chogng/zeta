use crate::CURRENT_SESSION_EVENT_SCHEMA_VERSION;
use crate::SessionStoreError;
use crate::StoredSessionEvent;
use crate::supports_session_event_schema_version;
use zeta_protocol::SessionId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionEventBatch {
    pub batch_id: String,
    pub session_id: SessionId,
    pub expected_sequence: u64,
    pub events: Vec<StoredSessionEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendSessionBatchResult {
    pub batch_id: String,
    pub committed_sequence: u64,
    pub event_count: usize,
}

/// Loads and atomically extends the authoritative structural history for one product Session.
///
/// Implementations must reject stale expected sequences, make a complete batch durable before
/// returning success, and never expose a partially committed batch from `load`.
pub trait SessionStore: Send + Sync {
    fn list_session_ids(&self) -> Result<Vec<SessionId>, SessionStoreError>;

    fn load(&self, session_id: &SessionId) -> Result<Vec<StoredSessionEvent>, SessionStoreError>;

    fn append_batch(
        &self,
        batch: &SessionEventBatch,
    ) -> Result<AppendSessionBatchResult, SessionStoreError>;
}

pub fn validate_session_append_batch(
    batch: &SessionEventBatch,
    actual_sequence: u64,
) -> Result<AppendSessionBatchResult, SessionStoreError> {
    if batch.expected_sequence != actual_sequence {
        return Err(SessionStoreError::SequenceConflict {
            expected: batch.expected_sequence,
            actual: actual_sequence,
        });
    }
    if batch.batch_id.trim().is_empty() {
        return Err(SessionStoreError::InvalidBatch(
            "batch ID must not be empty".into(),
        ));
    }
    if batch.events.is_empty() {
        return Err(SessionStoreError::InvalidBatch(
            "batch must contain at least one event".into(),
        ));
    }
    for (index, event) in batch.events.iter().enumerate() {
        let sequence = batch.expected_sequence + index as u64 + 1;
        if event.schema_version != CURRENT_SESSION_EVENT_SCHEMA_VERSION {
            return Err(SessionStoreError::InvalidBatch(
                "new events must use the current schema version".into(),
            ));
        }
        if event.session_id != batch.session_id
            || event.event.session_id() != &batch.session_id
            || event.sequence != sequence
        {
            return Err(SessionStoreError::InvalidBatch(
                "event Session identity or sequence does not match its batch".into(),
            ));
        }
        if batch.events[..index]
            .iter()
            .any(|existing| existing.event_id == event.event_id)
        {
            return Err(SessionStoreError::InvalidBatch(
                "event IDs must be unique within a batch".into(),
            ));
        }
    }
    Ok(AppendSessionBatchResult {
        batch_id: batch.batch_id.clone(),
        committed_sequence: batch.expected_sequence + batch.events.len() as u64,
        event_count: batch.events.len(),
    })
}

pub fn validate_session_history(
    session_id: &SessionId,
    events: &[StoredSessionEvent],
) -> Result<(), SessionStoreError> {
    let mut previous_sequence: Option<u64> = None;
    for (index, event) in events.iter().enumerate() {
        if &event.session_id != session_id || event.event.session_id() != session_id {
            return Err(SessionStoreError::Storage(
                "Session history contains an event for another Session".into(),
            ));
        }
        if !supports_session_event_schema_version(event.schema_version) {
            return Err(SessionStoreError::Storage(
                "Session history contains an unsupported event schema".into(),
            ));
        }
        let expected_sequence = match previous_sequence {
            None => 1,
            Some(previous) => previous.checked_add(1).ok_or_else(|| {
                SessionStoreError::Storage("Session history sequence overflowed".into())
            })?,
        };
        if event.sequence != expected_sequence {
            return Err(SessionStoreError::Storage(
                "Session history records are not contiguous and ordered".into(),
            ));
        }
        if events[..index]
            .iter()
            .any(|existing| existing.event_id == event.event_id)
        {
            return Err(SessionStoreError::Storage(
                "Session history contains duplicate event IDs".into(),
            ));
        }
        previous_sequence = Some(event.sequence);
    }
    Ok(())
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
