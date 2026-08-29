use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use std::path::Path;

use crate::{SqliteDurability, open_sqlite_database};

const STORAGE_SQLITE_SCHEMA_VERSION: u32 = 3;

pub(super) fn open(path: &Path) -> Result<Connection, String> {
    let mut connection = open_sqlite_database(path, SqliteDurability::Durable)?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS zeta_schema_migrations (
                 component TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             );",
        )
        .map_err(sql_error)?;
    let version = connection
        .query_row(
            "SELECT version FROM zeta_schema_migrations WHERE component = 'event-store'",
            [],
            |row| row.get::<_, u32>(0),
        )
        .optional()
        .map_err(sql_error)?;
    if let Some(version) = version
        && version > STORAGE_SQLITE_SCHEMA_VERSION
    {
        return Err(format!(
            "unsupported event-store SQLite schema version {version}"
        ));
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error)?;
    let locked_version = transaction
        .query_row(
            "SELECT version FROM zeta_schema_migrations WHERE component = 'event-store'",
            [],
            |row| row.get::<_, u32>(0),
        )
        .optional()
        .map_err(sql_error)?;
    match locked_version {
        None => transaction
            .execute_batch(
                "CREATE TABLE thread_streams (
                 thread_id TEXT PRIMARY KEY,
                 current_sequence INTEGER NOT NULL
             );
             CREATE TABLE thread_batches (
                 thread_id TEXT NOT NULL,
                 batch_id TEXT NOT NULL,
                 expected_sequence INTEGER NOT NULL,
                 event_count INTEGER NOT NULL,
                 PRIMARY KEY (thread_id, batch_id),
                 FOREIGN KEY (thread_id) REFERENCES thread_streams(thread_id)
             );
             CREATE TABLE thread_events (
                 thread_id TEXT NOT NULL,
                 sequence INTEGER NOT NULL,
                 event_id TEXT NOT NULL UNIQUE,
                 schema_version INTEGER NOT NULL,
                 envelope_json TEXT NOT NULL,
                 PRIMARY KEY (thread_id, sequence),
                 FOREIGN KEY (thread_id) REFERENCES thread_streams(thread_id)
             );
             CREATE TABLE session_streams (
                 session_id TEXT PRIMARY KEY,
                 current_sequence INTEGER NOT NULL
             );
             CREATE TABLE session_batches (
                 session_id TEXT NOT NULL,
                 batch_id TEXT NOT NULL,
                 expected_sequence INTEGER NOT NULL,
                 event_count INTEGER NOT NULL,
                 PRIMARY KEY (session_id, batch_id),
                 FOREIGN KEY (session_id) REFERENCES session_streams(session_id)
             );
             CREATE TABLE session_events (
                 session_id TEXT NOT NULL,
                 sequence INTEGER NOT NULL,
                 event_id TEXT NOT NULL UNIQUE,
                 schema_version INTEGER NOT NULL,
                 envelope_json TEXT NOT NULL,
                 PRIMARY KEY (session_id, sequence),
                 FOREIGN KEY (session_id) REFERENCES session_streams(session_id)
             );
             CREATE TABLE turn_change_sets (
                 change_set_id TEXT PRIMARY KEY,
                 thread_id TEXT NOT NULL,
                 revision INTEGER NOT NULL,
                 record_json TEXT NOT NULL
             );
             CREATE TABLE turn_change_commands (
                 command_id TEXT PRIMARY KEY,
                 fingerprint TEXT NOT NULL,
                 response_json TEXT NOT NULL
             );",
            )
            .map_err(sql_error)?,
        Some(1) => transaction
            .execute_batch(
                "CREATE TABLE turn_change_sets (
                     change_set_id TEXT PRIMARY KEY,
                     thread_id TEXT NOT NULL,
                     revision INTEGER NOT NULL,
                     record_json TEXT NOT NULL
                 );",
            )
            .map_err(sql_error)?,
        Some(2) => transaction
            .execute_batch(
                "CREATE TABLE turn_change_commands (
                     command_id TEXT PRIMARY KEY,
                     fingerprint TEXT NOT NULL,
                     response_json TEXT NOT NULL
                 );",
            )
            .map_err(sql_error)?,
        Some(STORAGE_SQLITE_SCHEMA_VERSION) => {}
        Some(version) => {
            return Err(format!(
                "unsupported event-store SQLite schema version {version}"
            ));
        }
    }
    transaction
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS turn_change_sets_thread_revision
             ON turn_change_sets(thread_id, revision);",
        )
        .map_err(sql_error)?;
    if locked_version.is_none() {
        transaction
            .execute(
                "INSERT INTO zeta_schema_migrations (component, version)
                 VALUES ('event-store', ?1)",
                [STORAGE_SQLITE_SCHEMA_VERSION],
            )
            .map_err(sql_error)?;
    } else if locked_version != Some(STORAGE_SQLITE_SCHEMA_VERSION) {
        transaction
            .execute(
                "UPDATE zeta_schema_migrations SET version = ?1
                 WHERE component = 'event-store'",
                [STORAGE_SQLITE_SCHEMA_VERSION],
            )
            .map_err(sql_error)?;
    }
    transaction.commit().map_err(sql_error)?;
    Ok(connection)
}

pub(super) fn to_sql_integer(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| "event sequence exceeds SQLite integer range".into())
}

pub(super) fn from_sql_integer(value: i64) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "event sequence is negative".into())
}

pub(super) fn sql_error(error: impl std::fmt::Display) -> String {
    format!("SQLite event-store error: {error}")
}
