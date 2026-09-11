use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use std::path::Path;

use crate::{SqliteDurability, open_sqlite_database};

const STORAGE_SQLITE_SCHEMA_VERSION: u32 = 7;

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
                "CREATE TABLE history_records (digest TEXT PRIMARY KEY, record_json TEXT NOT NULL);
                 CREATE TABLE thread_streams (
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
                 record_digest TEXT NOT NULL REFERENCES history_records(digest),
                 PRIMARY KEY (thread_id, sequence),
                 FOREIGN KEY (thread_id) REFERENCES thread_streams(thread_id)
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
                 );
                 CREATE TABLE turn_change_commands (
                     command_id TEXT PRIMARY KEY,
                     fingerprint TEXT NOT NULL,
                     response_json TEXT NOT NULL
                 );
                 DROP TABLE IF EXISTS session_events;
                 DROP TABLE IF EXISTS session_batches;
                 DROP TABLE IF EXISTS session_streams;",
            )
            .map_err(sql_error)?,
        Some(2) => transaction
            .execute_batch(
                "CREATE TABLE turn_change_commands (
                     command_id TEXT PRIMARY KEY,
                     fingerprint TEXT NOT NULL,
                     response_json TEXT NOT NULL
                 );
                 DROP TABLE IF EXISTS session_events;
                 DROP TABLE IF EXISTS session_batches;
                 DROP TABLE IF EXISTS session_streams;",
            )
            .map_err(sql_error)?,
        Some(3) => transaction
            .execute_batch(
                "DROP TABLE IF EXISTS session_events;
                 DROP TABLE IF EXISTS session_batches;
                 DROP TABLE IF EXISTS session_streams;",
            )
            .map_err(sql_error)?,
        Some(4) | Some(5) | Some(6) | Some(STORAGE_SQLITE_SCHEMA_VERSION) => {}
        Some(version) => {
            return Err(format!(
                "unsupported event-store SQLite schema version {version}"
            ));
        }
    }
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS thread_catalog (
                 thread_id TEXT PRIMARY KEY,
                 session_id TEXT NOT NULL,
                 requires_startup_recovery INTEGER NOT NULL,
                 record_json TEXT NOT NULL,
                 FOREIGN KEY (thread_id) REFERENCES thread_streams(thread_id)
             );
             CREATE INDEX IF NOT EXISTS thread_catalog_session
             ON thread_catalog(session_id, thread_id);
             CREATE INDEX IF NOT EXISTS thread_catalog_startup_recovery
             ON thread_catalog(requires_startup_recovery, session_id, thread_id);
             CREATE INDEX IF NOT EXISTS turn_change_sets_thread_revision
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
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS agents (
             agent_id TEXT PRIMARY KEY,
             created_at_unix_ms INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS agent_threads (
             thread_id TEXT PRIMARY KEY REFERENCES thread_streams(thread_id),
             agent_id TEXT NOT NULL REFERENCES agents(agent_id),
             session_id TEXT NOT NULL,
             spawn_parent_id TEXT,
             replacement_source_id TEXT UNIQUE,
             binding_json TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS agent_threads_agent ON agent_threads(agent_id, thread_id);
         CREATE INDEX IF NOT EXISTS agent_threads_parent ON agent_threads(spawn_parent_id, thread_id);"
    ).map_err(sql_error)?;
    if locked_version.is_some_and(|version| version < 6) {
        super::graph::migrate_bindings(&transaction).map_err(sql_error)?;
    }
    if locked_version != Some(STORAGE_SQLITE_SCHEMA_VERSION) {
        super::history::create_schema(&transaction)?;
        if locked_version.is_some() { super::history::migrate_records(&transaction)?; }
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
