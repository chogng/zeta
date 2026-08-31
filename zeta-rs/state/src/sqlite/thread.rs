use super::connection::{from_sql_integer, open, sql_error, to_sql_integer};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeta_history::StoredEvent;
use zeta_history::supports_stored_event_schema_version;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_thread_store::AppendBatchResult;
use zeta_thread_store::ThreadCatalogRecord;
use zeta_thread_store::ThreadEventBatch;
use zeta_thread_store::ThreadStore;
use zeta_thread_store::ThreadStoreError;
use zeta_thread_store::validate_append_batch;

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

    fn list_catalog(&self) -> Result<Vec<ThreadCatalogRecord>, ThreadStoreError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT catalog.thread_id,
                        catalog.session_id,
                        catalog.requires_startup_recovery,
                        catalog.record_json,
                        streams.current_sequence
                 FROM thread_catalog AS catalog
                 JOIN thread_streams AS streams ON streams.thread_id = catalog.thread_id
                 ORDER BY catalog.session_id, catalog.thread_id",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(storage_error)?
            .map(|row| {
                let (thread_id, session_id, requires_recovery, record_json, current_sequence) =
                    row.map_err(storage_error)?;
                let record = serde_json::from_str::<ThreadCatalogRecord>(&record_json)
                    .map_err(|error| ThreadStoreError::Storage(error.to_string()))?;
                let current_sequence =
                    from_sql_integer(current_sequence).map_err(ThreadStoreError::Storage)?;
                if record.thread.thread_id.as_str() != thread_id
                    || record.session_id.as_str() != session_id
                    || i64::from(record.requires_startup_recovery) != requires_recovery
                    || record.sequence != current_sequence
                {
                    return Err(ThreadStoreError::Storage(
                        "Thread catalog metadata disagrees with its stored record".into(),
                    ));
                }
                Ok(record)
            })
            .collect()
    }

    fn backfill_catalog(&self, record: &ThreadCatalogRecord) -> Result<(), ThreadStoreError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let current_sequence = transaction
            .query_row(
                "SELECT current_sequence FROM thread_streams WHERE thread_id = ?1",
                [record.thread.thread_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)
            .and_then(|value| from_sql_integer(value).map_err(ThreadStoreError::Storage))?;
        if current_sequence != record.sequence {
            return Err(ThreadStoreError::SequenceConflict {
                expected: record.sequence,
                actual: current_sequence,
            });
        }
        write_catalog(&transaction, record)?;
        transaction.commit().map_err(storage_error)
    }

    fn delete_session(&self, session_id: &SessionId) -> Result<Vec<ThreadId>, ThreadStoreError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let thread_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT thread_id FROM thread_catalog
                     WHERE session_id = ?1 ORDER BY thread_id",
                )
                .map_err(storage_error)?;
            statement
                .query_map([session_id.as_str()], |row| row.get::<_, String>(0))
                .map_err(storage_error)?
                .map(|row| {
                    ThreadId::new(row.map_err(storage_error)?)
                        .map_err(|error| ThreadStoreError::Storage(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        for thread_id in &thread_ids {
            transaction
                .execute(
                    "DELETE FROM turn_change_sets WHERE thread_id = ?1",
                    [thread_id.as_str()],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "DELETE FROM thread_catalog WHERE thread_id = ?1",
                    [thread_id.as_str()],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "DELETE FROM thread_events WHERE thread_id = ?1",
                    [thread_id.as_str()],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "DELETE FROM thread_batches WHERE thread_id = ?1",
                    [thread_id.as_str()],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "DELETE FROM thread_streams WHERE thread_id = ?1",
                    [thread_id.as_str()],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(thread_ids)
    }

    fn load(&self, thread_id: &ThreadId) -> Result<Vec<StoredEvent>, ThreadStoreError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(storage_error)?;
        let current_sequence = transaction
            .query_row(
                "SELECT current_sequence FROM thread_streams WHERE thread_id = ?1",
                [thread_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .map(from_sql_integer)
            .transpose()
            .map_err(ThreadStoreError::Storage)?;
        let events = {
            let mut statement = transaction
                .prepare(
                    "SELECT sequence, event_id, schema_version, envelope_json FROM thread_events
                     WHERE thread_id = ?1 ORDER BY sequence",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map([thread_id.as_str()], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, u32>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(storage_error)?;
            let mut events = Vec::new();
            for row in rows {
                let (sequence, event_id, schema_version, envelope) = row.map_err(storage_error)?;
                let sequence = from_sql_integer(sequence).map_err(ThreadStoreError::Storage)?;
                let event = serde_json::from_str::<StoredEvent>(&envelope)
                    .map_err(|error| ThreadStoreError::Storage(error.to_string()))?;
                if event.sequence != sequence
                    || event.event_id.0 != event_id
                    || event.schema_version != schema_version
                {
                    return Err(ThreadStoreError::Storage(
                        "Thread history row metadata disagrees with its envelope".into(),
                    ));
                }
                events.push(event);
            }
            events
        };
        validate_loaded(thread_id, &events)?;
        let loaded_sequence = events.last().map_or(0, |event| event.sequence);
        match current_sequence {
            Some(current_sequence) if current_sequence == loaded_sequence => {}
            None if events.is_empty() => {}
            _ => {
                return Err(ThreadStoreError::Storage(
                    "Thread stream sequence disagrees with its durable event tail".into(),
                ));
            }
        }
        transaction.commit().map_err(storage_error)?;
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
        write_catalog(&transaction, &batch.catalog)?;
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }
}

fn write_catalog(
    connection: &Connection,
    record: &ThreadCatalogRecord,
) -> Result<(), ThreadStoreError> {
    connection
        .execute(
            "INSERT INTO thread_catalog
             (thread_id, session_id, requires_startup_recovery, record_json)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(thread_id) DO UPDATE SET
                 session_id = excluded.session_id,
                 requires_startup_recovery = excluded.requires_startup_recovery,
                 record_json = excluded.record_json",
            params![
                record.thread.thread_id.as_str(),
                record.session_id.as_str(),
                record.requires_startup_recovery,
                serde_json::to_string(record)
                    .map_err(|error| ThreadStoreError::Storage(error.to_string()))?,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

impl SqliteThreadStore {
    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ThreadStoreError> {
        self.connection
            .lock()
            .map_err(|_| ThreadStoreError::Storage("Thread SQLite lock poisoned".into()))
    }
}

fn validate_loaded(thread_id: &ThreadId, events: &[StoredEvent]) -> Result<(), ThreadStoreError> {
    validate_history_records(thread_id, events)
}

fn validate_history_records(
    thread_id: &ThreadId,
    events: &[StoredEvent],
) -> Result<(), ThreadStoreError> {
    let mut previous_sequence: Option<u64> = None;
    for event in events {
        if &event.thread_id != thread_id || event.event.thread_id() != thread_id {
            return Err(ThreadStoreError::Storage(
                "Thread history contains an event for another Thread".into(),
            ));
        }
        if !supports_stored_event_schema_version(event.schema_version) {
            return Err(ThreadStoreError::Storage(
                "Thread history contains an unsupported event schema".into(),
            ));
        }
        let expected_sequence = match previous_sequence {
            None => 1,
            Some(previous) => previous.checked_add(1).ok_or_else(|| {
                ThreadStoreError::Storage("Thread history sequence overflowed".into())
            })?,
        };
        if event.sequence != expected_sequence {
            return Err(ThreadStoreError::Storage(
                "Thread history records are not contiguous and ordered".into(),
            ));
        }
        previous_sequence = Some(event.sequence);
    }
    Ok(())
}

fn storage_error(error: impl std::fmt::Display) -> ThreadStoreError {
    ThreadStoreError::Storage(sql_error(error))
}
