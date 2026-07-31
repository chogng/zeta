use super::connection::{from_sql_integer, open, sql_error, to_sql_integer};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeta_protocol::ThreadId;
use zeta_thread_store::{
    AppendBatchResult, StoredEvent, ThreadEventBatch, ThreadStore, ThreadStoreError,
    validate_append_batch,
};

/// SQLite implementation of the authoritative typed Thread event store.
pub struct SqliteThreadStore {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl SqliteThreadStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ThreadStoreError> {
        let path = path.into();
        let connection = open(&path).map_err(ThreadStoreError::Storage)?;
        Ok(Self {
            path,
            connection: Mutex::new(connection),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ThreadStore for SqliteThreadStore {
    fn list_thread_ids(&self) -> Result<Vec<ThreadId>, ThreadStoreError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT thread_id FROM thread_streams
                 WHERE current_sequence > 0 ORDER BY thread_id",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .map(|row| {
                ThreadId::new(row.map_err(storage_error)?)
                    .map_err(|error| ThreadStoreError::Storage(error.to_string()))
            })
            .collect()
    }

    fn load(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT envelope_json FROM thread_events
                 WHERE thread_id = ?1 ORDER BY sequence",
            )
            .map_err(storage_error)?;
        let events: Vec<StoredEvent> = statement
            .query_map([thread_id.as_str()], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .map(|row| {
                serde_json::from_str(&row.map_err(storage_error)?)
                    .map_err(|error| ThreadStoreError::Storage(error.to_string()))
            })
            .collect::<Result<_, _>>()?;
        validate_loaded(thread_id, &events)?;
        Ok(events)
    }

    fn append_batch(
        &self,
        batch: &ThreadEventBatch,
    ) -> Result<AppendBatchResult, ThreadStoreError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO thread_streams (thread_id, current_sequence)
                 VALUES (?1, 0)",
                [batch.thread_id.as_str()],
            )
            .map_err(storage_error)?;
        let actual = transaction
            .query_row(
                "SELECT current_sequence FROM thread_streams WHERE thread_id = ?1",
                [batch.thread_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)
            .and_then(|value| from_sql_integer(value).map_err(ThreadStoreError::Storage))?;
        let result = validate_append_batch(batch, actual)?;
        let duplicate_batch = transaction
            .query_row(
                "SELECT 1 FROM thread_batches WHERE thread_id = ?1 AND batch_id = ?2",
                params![batch.thread_id.as_str(), batch.batch_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if duplicate_batch {
            return Err(ThreadStoreError::InvalidBatch(
                "batch ID already exists".into(),
            ));
        }
        for event in &batch.events {
            let duplicate_event = transaction
                .query_row(
                    "SELECT 1 FROM thread_events WHERE event_id = ?1",
                    [&event.event_id.0],
                    |_| Ok(()),
                )
                .optional()
                .map_err(storage_error)?
                .is_some();
            if duplicate_event {
                return Err(ThreadStoreError::InvalidBatch(
                    "event ID already exists".into(),
                ));
            }
        }
        transaction
            .execute(
                "INSERT INTO thread_batches
                 (thread_id, batch_id, expected_sequence, event_count)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    batch.thread_id.as_str(),
                    batch.batch_id,
                    to_sql_integer(batch.expected_sequence).map_err(ThreadStoreError::Storage)?,
                    to_sql_integer(batch.events.len() as u64).map_err(ThreadStoreError::Storage)?,
                ],
            )
            .map_err(storage_error)?;
        for event in &batch.events {
            transaction
                .execute(
                    "INSERT INTO thread_events
                     (thread_id, sequence, event_id, schema_version, envelope_json)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        batch.thread_id.as_str(),
                        to_sql_integer(event.sequence).map_err(ThreadStoreError::Storage)?,
                        event.event_id.0,
                        event.schema_version,
                        serde_json::to_string(event)
                            .map_err(|error| ThreadStoreError::Storage(error.to_string()))?,
                    ],
                )
                .map_err(storage_error)?;
        }
        let updated = transaction
            .execute(
                "UPDATE thread_streams SET current_sequence = ?1
                 WHERE thread_id = ?2 AND current_sequence = ?3",
                params![
                    to_sql_integer(result.committed_sequence).map_err(ThreadStoreError::Storage)?,
                    batch.thread_id.as_str(),
                    to_sql_integer(batch.expected_sequence).map_err(ThreadStoreError::Storage)?,
                ],
            )
            .map_err(storage_error)?;
        if updated != 1 {
            return Err(ThreadStoreError::SequenceConflict {
                expected: batch.expected_sequence,
                actual,
            });
        }
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }
}

impl SqliteThreadStore {
    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ThreadStoreError> {
        self.connection
            .lock()
            .map_err(|_| ThreadStoreError::Storage("Thread SQLite lock poisoned".into()))
    }
}

fn validate_loaded(thread_id: &ThreadId, events: &[StoredEvent]) -> Result<(), ThreadStoreError> {
    if events.is_empty() {
        return Ok(());
    }
    validate_append_batch(
        &ThreadEventBatch {
            batch_id: "sqlite-recovery".into(),
            thread_id: thread_id.clone(),
            expected_sequence: 0,
            events: events.to_vec(),
        },
        0,
    )
    .map(|_| ())
}

fn storage_error(error: impl std::fmt::Display) -> ThreadStoreError {
    ThreadStoreError::Storage(sql_error(error))
}
