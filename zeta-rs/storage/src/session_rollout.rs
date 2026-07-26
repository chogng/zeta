use crate::event_stream::{append_batch, read_batches};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use zeta_protocol::SessionId;
use zeta_session_store::{
    AppendSessionBatchResult, SessionEventBatch, SessionStore, SessionStoreError,
    StoredSessionEvent, validate_session_append_batch,
};

const STREAM_KIND: &str = "session";

/// Typed Session facade over the shared event-stream rollout engine.
pub struct SessionRolloutStore {
    root: PathBuf,
    write_lock: Mutex<()>,
}

impl SessionRolloutStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, SessionStoreError> {
        let root = root.into().join("streams").join(STREAM_KIND);
        fs::create_dir_all(&root).map_err(|error| SessionStoreError::Storage(error.to_string()))?;
        Ok(Self {
            root,
            write_lock: Mutex::new(()),
        })
    }

    fn session_path(&self, session_id: &SessionId) -> PathBuf {
        self.root
            .join(format!("{}.rollout", encode_hex(session_id.as_str())))
    }
}

impl SessionStore for SessionRolloutStore {
    fn list_session_ids(&self) -> Result<Vec<SessionId>, SessionStoreError> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(|error| SessionStoreError::Storage(error.to_string()))?
        {
            let path = entry
                .map_err(|error| SessionStoreError::Storage(error.to_string()))?
                .path();
            if path
                .extension()
                .is_some_and(|extension| extension == "rollout")
            {
                ids.extend(
                    read_batches::<StoredSessionEvent>(&path, STREAM_KIND)
                        .map_err(SessionStoreError::Storage)?
                        .into_iter()
                        .flat_map(|batch| batch.events)
                        .map(|event| event.session_id),
                );
            }
        }
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    fn load(&self, session_id: &SessionId) -> Result<Vec<StoredSessionEvent>, SessionStoreError> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_and_validate(&path)
    }

    fn append_batch(
        &self,
        batch: &SessionEventBatch,
    ) -> Result<AppendSessionBatchResult, SessionStoreError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| SessionStoreError::Storage("Session rollout lock poisoned".into()))?;
        let path = self.session_path(&batch.session_id);
        let existing = if path.exists() {
            read_and_validate(&path)?
        } else {
            Vec::new()
        };
        let actual_sequence = existing.last().map_or(0, |event| event.sequence);
        let result = validate_session_append_batch(batch, actual_sequence)?;
        let batches = read_batches::<StoredSessionEvent>(&path, STREAM_KIND)
            .map_err(SessionStoreError::Storage)?;
        if batches
            .iter()
            .any(|existing| existing.batch_id == batch.batch_id)
        {
            return Err(SessionStoreError::InvalidBatch(
                "batch ID already exists".into(),
            ));
        }
        if batch.events.iter().any(|event| {
            existing
                .iter()
                .any(|existing| existing.event_id == event.event_id)
        }) {
            return Err(SessionStoreError::InvalidBatch(
                "event ID already exists".into(),
            ));
        }
        append_batch(
            &path,
            STREAM_KIND,
            &batch.batch_id,
            batch.session_id.as_str(),
            batch.expected_sequence,
            &batch.events,
        )
        .map_err(SessionStoreError::Storage)?;
        Ok(result)
    }
}

fn read_and_validate(path: &std::path::Path) -> Result<Vec<StoredSessionEvent>, SessionStoreError> {
    let mut events = Vec::new();
    let mut sequence = 0;
    for batch in
        read_batches::<StoredSessionEvent>(path, STREAM_KIND).map_err(SessionStoreError::Storage)?
    {
        let session_id = SessionId::new(batch.stream_id)
            .map_err(|error| SessionStoreError::InvalidBatch(error.to_string()))?;
        let typed = SessionEventBatch {
            batch_id: batch.batch_id,
            session_id,
            expected_sequence: batch.expected_sequence,
            events: batch.events,
        };
        validate_session_append_batch(&typed, sequence)?;
        sequence += typed.events.len() as u64;
        events.extend(typed.events);
    }
    Ok(events)
}

fn encode_hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "session_rollout_tests.rs"]
mod tests;
