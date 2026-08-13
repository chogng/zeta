use super::connection::{from_sql_integer, open, sql_error, to_sql_integer};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeta_protocol::SessionId;
use zeta_session_store::AppendSessionBatchResult;
use zeta_session_store::SessionEventBatch;
use zeta_session_store::SessionStore;
use zeta_session_store::SessionStoreError;
use zeta_session_store::StoredSessionEvent;
use zeta_session_store::validate_session_append_batch;
use zeta_session_store::validate_session_history;

/// SQLite implementation of the authoritative typed Session event store.
pub struct SqliteSessionStore {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl SqliteSessionStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SessionStoreError> {
        let path = path.into();
        let connection = open(&path).map_err(SessionStoreError::Storage)?;
        Ok(Self {
            path,
            connection: Mutex::new(connection),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SessionStore for SqliteSessionStore {
    fn list_session_ids(&self) -> Result<Vec<SessionId>, SessionStoreError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT session_id FROM session_streams
                 WHERE current_sequence > 0 ORDER BY session_id",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .map(|row| {
                SessionId::new(row.map_err(storage_error)?)
                    .map_err(|error| SessionStoreError::Storage(error.to_string()))
            })
            .collect()
    }

    fn load(&self, session_id: &SessionId) -> Result<Vec<StoredSessionEvent>, SessionStoreError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(storage_error)?;
        let current_sequence = transaction
            .query_row(
                "SELECT current_sequence FROM session_streams WHERE session_id = ?1",
                [session_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .map(from_sql_integer)
            .transpose()
            .map_err(SessionStoreError::Storage)?;
        let events = {
            let mut statement = transaction
                .prepare(
                    "SELECT sequence, event_id, schema_version, envelope_json FROM session_events
                     WHERE session_id = ?1 ORDER BY sequence",
                )
                .map_err(storage_error)?;
            let rows = statement
                .query_map([session_id.as_str()], |row| {
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
                let sequence = from_sql_integer(sequence).map_err(SessionStoreError::Storage)?;
                let event = serde_json::from_str::<StoredSessionEvent>(&envelope)
                    .map_err(|error| SessionStoreError::Storage(error.to_string()))?;
                if event.sequence != sequence
                    || event.event_id.0 != event_id
                    || event.schema_version != schema_version
                {
                    return Err(SessionStoreError::Storage(
                        "Session history row metadata disagrees with its envelope".into(),
                    ));
                }
                events.push(event);
            }
            events
        };
        validate_session_history(session_id, &events)?;
        let loaded_sequence = events.last().map_or(0, |event| event.sequence);
        match current_sequence {
            Some(current_sequence) if current_sequence == loaded_sequence => {}
            None if events.is_empty() => {}
            _ => {
                return Err(SessionStoreError::Storage(
                    "Session stream sequence disagrees with its durable event tail".into(),
                ));
            }
        }
        transaction.commit().map_err(storage_error)?;
        Ok(events)
    }

    fn append_batch(
        &self,
        batch: &SessionEventBatch,
    ) -> Result<AppendSessionBatchResult, SessionStoreError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO session_streams (session_id, current_sequence)
                 VALUES (?1, 0)",
                [batch.session_id.as_str()],
            )
            .map_err(storage_error)?;
        let actual = transaction
            .query_row(
                "SELECT current_sequence FROM session_streams WHERE session_id = ?1",
                [batch.session_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)
            .and_then(|value| from_sql_integer(value).map_err(SessionStoreError::Storage))?;
        let result = validate_session_append_batch(batch, actual)?;
        let duplicate_batch = transaction
            .query_row(
                "SELECT 1 FROM session_batches WHERE session_id = ?1 AND batch_id = ?2",
                params![batch.session_id.as_str(), batch.batch_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        if duplicate_batch {
            return Err(SessionStoreError::InvalidBatch(
                "batch ID already exists".into(),
            ));
        }
        for event in &batch.events {
            let duplicate_event = transaction
                .query_row(
                    "SELECT 1 FROM session_events WHERE event_id = ?1",
                    [&event.event_id.0],
                    |_| Ok(()),
                )
                .optional()
                .map_err(storage_error)?
                .is_some();
            if duplicate_event {
                return Err(SessionStoreError::InvalidBatch(
                    "event ID already exists".into(),
                ));
            }
        }
        transaction
            .execute(
                "INSERT INTO session_batches
                 (session_id, batch_id, expected_sequence, event_count)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    batch.session_id.as_str(),
                    batch.batch_id,
                    to_sql_integer(batch.expected_sequence).map_err(SessionStoreError::Storage)?,
                    to_sql_integer(batch.events.len() as u64)
                        .map_err(SessionStoreError::Storage)?,
                ],
            )
            .map_err(storage_error)?;
        for event in &batch.events {
            transaction
                .execute(
                    "INSERT INTO session_events
                     (session_id, sequence, event_id, schema_version, envelope_json)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        batch.session_id.as_str(),
                        to_sql_integer(event.sequence).map_err(SessionStoreError::Storage)?,
                        event.event_id.0,
                        event.schema_version,
                        serde_json::to_string(event)
                            .map_err(|error| SessionStoreError::Storage(error.to_string()))?,
                    ],
                )
                .map_err(storage_error)?;
        }
        let updated = transaction
            .execute(
                "UPDATE session_streams SET current_sequence = ?1
                 WHERE session_id = ?2 AND current_sequence = ?3",
                params![
                    to_sql_integer(result.committed_sequence)
                        .map_err(SessionStoreError::Storage)?,
                    batch.session_id.as_str(),
                    to_sql_integer(batch.expected_sequence).map_err(SessionStoreError::Storage)?,
                ],
            )
            .map_err(storage_error)?;
        if updated != 1 {
            return Err(SessionStoreError::SequenceConflict {
                expected: batch.expected_sequence,
                actual,
            });
        }
        transaction.commit().map_err(storage_error)?;
        Ok(result)
    }
}

impl SqliteSessionStore {
    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, SessionStoreError> {
        self.connection
            .lock()
            .map_err(|_| SessionStoreError::Storage("Session SQLite lock poisoned".into()))
    }
}

fn storage_error(error: impl std::fmt::Display) -> SessionStoreError {
    SessionStoreError::Storage(sql_error(error))
}
