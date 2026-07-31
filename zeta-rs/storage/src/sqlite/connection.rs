use rusqlite::{Connection, ErrorCode, OptionalExtension, TransactionBehavior};
use std::fs;
use std::path::Path;
use std::time::Duration;

const STORAGE_SQLITE_SCHEMA_VERSION: u32 = 1;

pub(super) fn open(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    prepare_private_database_file(path)?;
    let mut connection = Connection::open(path).map_err(sql_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(sql_error)?;
    enable_wal(&connection)?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = FULL;
             CREATE TABLE IF NOT EXISTS zeta_schema_migrations (
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
    if let Some(version) = version {
        if version != STORAGE_SQLITE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported event-store SQLite schema version {version}"
            ));
        }
        return Ok(connection);
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
    if let Some(version) = locked_version {
        if version != STORAGE_SQLITE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported event-store SQLite schema version {version}"
            ));
        }
        transaction.commit().map_err(sql_error)?;
        return Ok(connection);
    }
    transaction
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
             );",
        )
        .map_err(sql_error)?;
    transaction
        .execute(
            "INSERT INTO zeta_schema_migrations (component, version)
             VALUES ('event-store', ?1)",
            [STORAGE_SQLITE_SCHEMA_VERSION],
        )
        .map_err(sql_error)?;
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

fn enable_wal(connection: &Connection) -> Result<(), String> {
    for _ in 0..100 {
        match connection.query_row("PRAGMA journal_mode = WAL", [], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(_) => return Ok(()),
            Err(rusqlite::Error::SqliteFailure(error, _))
                if matches!(
                    error.code,
                    ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                ) =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(sql_error(error)),
        }
    }
    Err("SQLite event-store database remained locked while enabling WAL".into())
}

fn prepare_private_database_file(path: &Path) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
