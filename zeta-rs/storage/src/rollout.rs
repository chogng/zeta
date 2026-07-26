use crate::event_stream::{append_batch, read_batches};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use zeta_core::CoreError;
use zeta_protocol::ThreadId;
use zeta_thread_store::{
    AppendBatchResult, StoredEvent, ThreadEventBatch, ThreadStore, ThreadStoreError,
    validate_append_batch,
};

const STREAM_KIND: &str = "thread";

/// Maps each Thread to its own logical stream in the shared rollout format.
pub struct ThreadRolloutStore {
    root: PathBuf,
    write_lock: Mutex<()>,
}

impl ThreadRolloutStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, CoreError> {
        let root = root.into().join("streams").join(STREAM_KIND);
        fs::create_dir_all(&root).map_err(io_error)?;
        Ok(Self {
            root,
            write_lock: Mutex::new(()),
        })
    }

    pub fn read_thread(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, CoreError> {
        self.load(thread_id).map_err(CoreError::ThreadStore)
    }

    pub fn all_thread_events(&self) -> Result<Vec<Vec<StoredEvent>>, CoreError> {
        let mut rollouts = Vec::new();
        for thread_id in self.list_thread_ids().map_err(CoreError::ThreadStore)? {
            let events = self.load(&thread_id).map_err(CoreError::ThreadStore)?;
            if !events.is_empty() {
                rollouts.push(events);
            }
        }
        Ok(rollouts)
    }

    pub fn rebuild_sqlite_projection(
        &self,
        database_path: impl AsRef<Path>,
    ) -> Result<(), CoreError> {
        let database_path = database_path.as_ref();
        recreate_projection(database_path)?;
        for events in self.all_thread_events()? {
            insert_projection_events(database_path, events)?;
        }
        Ok(())
    }

    fn thread_path(&self, thread_id: &ThreadId) -> PathBuf {
        self.root
            .join(format!("{}.rollout", encode_hex(thread_id.as_str())))
    }
}

impl ThreadStore for ThreadRolloutStore {
    fn list_thread_ids(&self) -> Result<Vec<ThreadId>, ThreadStoreError> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(|error| ThreadStoreError::Storage(error.to_string()))?
        {
            let path = entry
                .map_err(|error| ThreadStoreError::Storage(error.to_string()))?
                .path();
            if path
                .extension()
                .is_some_and(|extension| extension == "rollout")
            {
                ids.extend(
                    read_and_validate(&path)?
                        .into_iter()
                        .map(|event| event.thread_id),
                );
            }
        }
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    fn load(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError> {
        let path = self.thread_path(thread_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_and_validate(&path)
    }

    fn append_batch(
        &self,
        batch: &ThreadEventBatch,
    ) -> Result<AppendBatchResult, ThreadStoreError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| ThreadStoreError::Storage("rollout write lock poisoned".into()))?;
        append_thread_batch(&self.thread_path(&batch.thread_id), batch)
    }
}

fn append_thread_batch(
    path: &Path,
    batch: &ThreadEventBatch,
) -> Result<AppendBatchResult, ThreadStoreError> {
    let existing = if path.exists() {
        read_and_validate(path)?
    } else {
        Vec::new()
    };
    let actual_sequence = existing
        .iter()
        .rev()
        .find(|event| event.thread_id == batch.thread_id)
        .map_or(0, |event| event.sequence);
    let result = validate_append_batch(batch, actual_sequence)?;
    let batches =
        read_batches::<StoredEvent>(path, STREAM_KIND).map_err(ThreadStoreError::Storage)?;
    if batches
        .iter()
        .any(|existing| existing.batch_id == batch.batch_id)
    {
        return Err(ThreadStoreError::InvalidBatch(
            "batch ID already exists".into(),
        ));
    }
    if batch.events.iter().any(|event| {
        existing
            .iter()
            .any(|existing| existing.event_id == event.event_id)
    }) {
        return Err(ThreadStoreError::InvalidBatch(
            "event ID already exists".into(),
        ));
    }
    append_batch(
        path,
        STREAM_KIND,
        &batch.batch_id,
        batch.thread_id.as_str(),
        batch.expected_sequence,
        &batch.events,
    )
    .map_err(ThreadStoreError::Storage)?;
    Ok(result)
}

fn read_and_validate(path: &Path) -> Result<Vec<StoredEvent>, ThreadStoreError> {
    let mut all_events = Vec::new();
    let mut sequences = BTreeMap::<ThreadId, u64>::new();
    let mut batch_ids = BTreeSet::new();
    for batch in
        read_batches::<StoredEvent>(path, STREAM_KIND).map_err(ThreadStoreError::Storage)?
    {
        if !batch_ids.insert(batch.batch_id.clone()) {
            return Err(ThreadStoreError::InvalidBatch(
                "batch ID already exists".into(),
            ));
        }
        let thread_id = ThreadId::new(batch.stream_id)
            .map_err(|error| ThreadStoreError::InvalidBatch(error.to_string()))?;
        let actual = *sequences.get(&thread_id).unwrap_or(&0);
        let typed = ThreadEventBatch {
            batch_id: batch.batch_id,
            thread_id: thread_id.clone(),
            expected_sequence: batch.expected_sequence,
            events: batch.events,
        };
        validate_append_batch(&typed, actual)?;
        sequences.insert(thread_id, actual + typed.events.len() as u64);
        all_events.extend(typed.events);
    }
    Ok(all_events)
}

fn recreate_projection(database_path: &Path) -> Result<(), CoreError> {
    if database_path.exists() {
        fs::remove_file(database_path).map_err(io_error)?;
    }
    run_sql(
        database_path,
        "CREATE TABLE events (event_id TEXT PRIMARY KEY, sequence INTEGER NOT NULL, thread_id TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, occurred_at INTEGER NOT NULL);",
    )
}

fn insert_projection_events(
    database_path: &Path,
    events: Vec<StoredEvent>,
) -> Result<(), CoreError> {
    for event in events {
        let payload = serde_json::to_string(&event.event)
            .map_err(|error| CoreError::Journal(error.to_string()))?;
        run_sql(
            database_path,
            &format!(
                "INSERT INTO events (event_id, sequence, thread_id, kind, payload, occurred_at) VALUES ({}, {}, {}, {}, {}, {});",
                quoted(&event.event_id.0),
                event.sequence,
                quoted(event.thread_id.as_str()),
                quoted(event.event.kind()),
                quoted(&payload),
                event.recorded_at.0
            ),
        )?;
    }
    Ok(())
}

fn run_sql(database_path: &Path, sql: &str) -> Result<(), CoreError> {
    let output = Command::new("sqlite3")
        .arg(database_path)
        .arg(sql)
        .output()
        .map_err(io_error)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CoreError::Journal(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ))
    }
}

fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn encode_hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn io_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::Journal(error.to_string())
}
