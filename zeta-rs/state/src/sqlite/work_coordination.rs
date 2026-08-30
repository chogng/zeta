use super::connection::from_sql_integer;
use super::connection::to_sql_integer;
use crate::SqliteDurability;
use crate::open_sqlite_database;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::TransactionBehavior;
use rusqlite::params;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use zeta_protocol::CommandId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunCommit;
use zeta_work_coordination::WorkRunStore;
use zeta_work_coordination::WorkRunStoreError;
use zeta_work_coordination::WorkRunStoreOutcome;

const WORK_COORDINATION_SCHEMA_VERSION: u32 = 2;
const WORK_COORDINATION_COMPONENT: &str = "work-coordination";

/// SQLite implementation of complete WorkRun records and retry-safe command receipts.
pub struct SqliteWorkRunStore {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl SqliteWorkRunStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, WorkRunStoreError> {
        let path = path.into();
        let mut connection = open_sqlite_database(&path, SqliteDurability::Durable)
            .map_err(WorkRunStoreError::Storage)?;
        initialize(&mut connection)?;
        validate_writer_leases(&connection)?;
        Ok(Self {
            path,
            connection: Mutex::new(connection),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, WorkRunStoreError> {
        self.connection
            .lock()
            .map_err(|_| WorkRunStoreError::Storage("WorkRun SQLite lock poisoned".into()))
    }
}

impl WorkRunStore for SqliteWorkRunStore {
    fn list(&self) -> Result<Vec<WorkRun>, WorkRunStoreError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT work_run_id, revision, record_json FROM work_runs
                 ORDER BY work_run_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage_error)?;
        let mut runs = Vec::new();
        for row in rows {
            let (work_run_id, revision, record) = row.map_err(storage_error)?;
            let work_run_id = WorkRunId::new(work_run_id)
                .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?;
            runs.push(deserialize_run(&work_run_id, revision, &record)?);
        }
        Ok(runs)
    }

    fn load(&self, work_run_id: &WorkRunId) -> Result<WorkRun, WorkRunStoreError> {
        let connection = self.connection()?;
        let row = connection
            .query_row(
                "SELECT revision, record_json FROM work_runs WHERE work_run_id = ?1",
                [work_run_id.as_str()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| WorkRunStoreError::NotFound(work_run_id.to_string()))?;
        deserialize_run(work_run_id, row.0, &row.1)
    }

    fn load_command(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<WorkRunCommit>, WorkRunStoreError> {
        let connection = self.connection()?;
        load_command(&connection, command_id)
    }

    fn commit(&self, commit: &WorkRunCommit) -> Result<WorkRunStoreOutcome, WorkRunStoreError> {
        validate_commit(commit)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        validate_writer_leases(&transaction)?;
        if let Some(existing) = load_command(&transaction, &commit.request.command_id)? {
            if existing.request != commit.request {
                return Err(WorkRunStoreError::CommandConflict);
            }
            transaction.commit().map_err(storage_error)?;
            return Ok(WorkRunStoreOutcome::Replayed(existing.result));
        }
        let actual = transaction
            .query_row(
                "SELECT revision FROM work_runs WHERE work_run_id = ?1",
                [commit.request.work_run_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_error)?
            .map(from_sql_integer)
            .transpose()
            .map_err(WorkRunStoreError::Storage)?
            .unwrap_or(0);
        if actual != commit.request.expected_revision {
            return Err(WorkRunStoreError::RevisionConflict {
                expected: commit.request.expected_revision,
                actual,
            });
        }
        let record = serialize(&commit.result)?;
        if actual == 0 {
            transaction
                .execute(
                    "INSERT INTO work_runs (work_run_id, revision, record_json)
                     VALUES (?1, ?2, ?3)",
                    params![
                        commit.result.work_run_id.as_str(),
                        to_sql_integer(commit.result.revision)
                            .map_err(WorkRunStoreError::Storage)?,
                        record,
                    ],
                )
                .map_err(storage_error)?;
        } else {
            let updated = transaction
                .execute(
                    "UPDATE work_runs SET revision = ?1, record_json = ?2
                     WHERE work_run_id = ?3 AND revision = ?4",
                    params![
                        to_sql_integer(commit.result.revision)
                            .map_err(WorkRunStoreError::Storage)?,
                        record,
                        commit.result.work_run_id.as_str(),
                        to_sql_integer(actual).map_err(WorkRunStoreError::Storage)?,
                    ],
                )
                .map_err(storage_error)?;
            if updated != 1 {
                return Err(WorkRunStoreError::RevisionConflict {
                    expected: commit.request.expected_revision,
                    actual,
                });
            }
        }
        replace_writer_leases(&transaction, &commit.result)?;
        transaction
            .execute(
                "INSERT INTO work_run_commands
                 (command_id, work_run_id, expected_revision, request_json,
                  result_revision, result_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    commit.request.command_id.as_str(),
                    commit.request.work_run_id.as_str(),
                    to_sql_integer(commit.request.expected_revision)
                        .map_err(WorkRunStoreError::Storage)?,
                    serialize(&commit.request)?,
                    to_sql_integer(commit.result.revision).map_err(WorkRunStoreError::Storage)?,
                    serialize(&commit.result)?,
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(WorkRunStoreOutcome::Applied)
    }
}

fn initialize(connection: &mut Connection) -> Result<(), WorkRunStoreError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS zeta_schema_migrations (
                 component TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             );",
        )
        .map_err(storage_error)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let version = transaction
        .query_row(
            "SELECT version FROM zeta_schema_migrations WHERE component = ?1",
            [WORK_COORDINATION_COMPONENT],
            |row| row.get::<_, u32>(0),
        )
        .optional()
        .map_err(storage_error)?;
    match version {
        None => {
            transaction
                .execute_batch(
                    "CREATE TABLE work_runs (
                         work_run_id TEXT PRIMARY KEY,
                         revision INTEGER NOT NULL,
                         record_json TEXT NOT NULL
                     );
                     CREATE TABLE work_run_commands (
                         command_id TEXT PRIMARY KEY,
                         work_run_id TEXT NOT NULL,
                         expected_revision INTEGER NOT NULL,
                         request_json TEXT NOT NULL,
                         result_revision INTEGER NOT NULL,
                         result_json TEXT NOT NULL,
                         FOREIGN KEY (work_run_id) REFERENCES work_runs(work_run_id)
                     );
                     CREATE INDEX work_run_commands_run_revision
                     ON work_run_commands(work_run_id, result_revision);
                     CREATE TABLE work_attempt_writers (
                         thread_id TEXT PRIMARY KEY,
                         work_run_id TEXT NOT NULL,
                         attempt_id TEXT NOT NULL,
                         execution_id TEXT NOT NULL,
                         FOREIGN KEY (work_run_id) REFERENCES work_runs(work_run_id)
                     );
                     CREATE UNIQUE INDEX work_attempt_writer_attempt
                     ON work_attempt_writers(work_run_id, attempt_id);",
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "INSERT INTO zeta_schema_migrations (component, version) VALUES (?1, ?2)",
                    params![
                        WORK_COORDINATION_COMPONENT,
                        WORK_COORDINATION_SCHEMA_VERSION
                    ],
                )
                .map_err(storage_error)?;
        }
        Some(1) => {
            transaction
                .execute_batch(
                    "CREATE TABLE work_attempt_writers (
                         thread_id TEXT PRIMARY KEY,
                         work_run_id TEXT NOT NULL,
                         attempt_id TEXT NOT NULL,
                         execution_id TEXT NOT NULL,
                         FOREIGN KEY (work_run_id) REFERENCES work_runs(work_run_id)
                     );
                     CREATE UNIQUE INDEX work_attempt_writer_attempt
                     ON work_attempt_writers(work_run_id, attempt_id);",
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "UPDATE zeta_schema_migrations SET version = ?1 WHERE component = ?2",
                    params![
                        WORK_COORDINATION_SCHEMA_VERSION,
                        WORK_COORDINATION_COMPONENT
                    ],
                )
                .map_err(storage_error)?;
        }
        Some(WORK_COORDINATION_SCHEMA_VERSION) => {}
        Some(version) => {
            return Err(WorkRunStoreError::Storage(format!(
                "unsupported WorkRun SQLite schema version {version}"
            )));
        }
    }
    transaction.commit().map_err(storage_error)
}

fn replace_writer_leases(connection: &Connection, run: &WorkRun) -> Result<(), WorkRunStoreError> {
    for attempt in run.active_writers() {
        attempt.execution_id.as_ref().ok_or_else(|| {
            WorkRunStoreError::Storage("active WorkAttempt omitted its execution identity".into())
        })?;
        let existing = connection
            .query_row(
                "SELECT work_run_id, attempt_id FROM work_attempt_writers WHERE thread_id = ?1",
                [attempt.thread_id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        if let Some((work_run_id, attempt_id)) = existing
            && work_run_id != run.work_run_id.as_str()
        {
            return Err(WorkRunStoreError::ThreadBusy {
                thread_id: attempt.thread_id.clone(),
                work_run_id: WorkRunId::new(work_run_id)
                    .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?,
                attempt_id: WorkAttemptId::new(attempt_id)
                    .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?,
            });
        }
    }
    connection
        .execute(
            "DELETE FROM work_attempt_writers WHERE work_run_id = ?1",
            [run.work_run_id.as_str()],
        )
        .map_err(storage_error)?;
    for attempt in run.active_writers() {
        let execution_id = attempt.execution_id.as_ref().ok_or_else(|| {
            WorkRunStoreError::Storage("active WorkAttempt omitted its execution identity".into())
        })?;
        connection
            .execute(
                "INSERT INTO work_attempt_writers
                 (thread_id, work_run_id, attempt_id, execution_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    attempt.thread_id.as_str(),
                    run.work_run_id.as_str(),
                    attempt.attempt_id.as_str(),
                    execution_id.as_str(),
                ],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn validate_writer_leases(connection: &Connection) -> Result<(), WorkRunStoreError> {
    let mut expected = BTreeMap::new();
    let mut statement = connection
        .prepare("SELECT work_run_id, revision, record_json FROM work_runs")
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(storage_error)?;
    for row in rows {
        let (work_run_id, revision, record) = row.map_err(storage_error)?;
        let work_run_id = WorkRunId::new(work_run_id)
            .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?;
        let run = deserialize_run(&work_run_id, revision, &record)?;
        for attempt in run.active_writers() {
            let execution_id = attempt.execution_id.as_ref().ok_or_else(|| {
                WorkRunStoreError::Storage(
                    "active WorkAttempt omitted its execution identity".into(),
                )
            })?;
            if let Some((other_run, other_attempt, _)) = expected.insert(
                attempt.thread_id.clone(),
                (
                    run.work_run_id.clone(),
                    attempt.attempt_id.clone(),
                    execution_id.clone(),
                ),
            ) {
                return Err(WorkRunStoreError::Storage(format!(
                    "Thread {} has active attempts {} in {} and {} in {}",
                    attempt.thread_id,
                    other_attempt,
                    other_run,
                    attempt.attempt_id,
                    run.work_run_id
                )));
            }
        }
    }
    drop(statement);

    let mut actual = BTreeMap::new();
    let mut statement = connection
        .prepare(
            "SELECT thread_id, work_run_id, attempt_id, execution_id
             FROM work_attempt_writers",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(storage_error)?;
    for row in rows {
        let (thread_id, work_run_id, attempt_id, execution_id) = row.map_err(storage_error)?;
        let thread_id = ThreadId::new(thread_id)
            .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?;
        let lease = (
            WorkRunId::new(work_run_id)
                .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?,
            WorkAttemptId::new(attempt_id)
                .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?,
            WorkExecutionId::new(execution_id)
                .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?,
        );
        if actual.insert(thread_id, lease).is_some() {
            return Err(WorkRunStoreError::Storage(
                "writer lease table repeats a Thread identity".into(),
            ));
        }
    }
    if actual != expected {
        return Err(WorkRunStoreError::Storage(
            "writer lease table disagrees with durable WorkRun state".into(),
        ));
    }
    Ok(())
}

fn load_command(
    connection: &Connection,
    command_id: &CommandId,
) -> Result<Option<WorkRunCommit>, WorkRunStoreError> {
    let row = connection
        .query_row(
            "SELECT work_run_id, expected_revision, request_json,
                    result_revision, result_json
             FROM work_run_commands WHERE command_id = ?1",
            [command_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    row.map(
        |(work_run_id, expected_revision, request_json, result_revision, result_json)| {
            let request = serde_json::from_str::<zeta_work_coordination::WorkRunCommandRequest>(
                &request_json,
            )
            .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?;
            let result = serde_json::from_str::<WorkRun>(&result_json)
                .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?;
            let expected_revision =
                from_sql_integer(expected_revision).map_err(WorkRunStoreError::Storage)?;
            let result_revision =
                from_sql_integer(result_revision).map_err(WorkRunStoreError::Storage)?;
            if request.command_id != *command_id
                || request.work_run_id.as_str() != work_run_id
                || request.expected_revision != expected_revision
                || result.work_run_id != request.work_run_id
                || result.revision != result_revision
            {
                return Err(WorkRunStoreError::Storage(
                    "WorkRun command row metadata disagrees with its record".into(),
                ));
            }
            validate_run(&result)?;
            Ok(WorkRunCommit { request, result })
        },
    )
    .transpose()
}

fn validate_commit(commit: &WorkRunCommit) -> Result<(), WorkRunStoreError> {
    let next_revision = commit
        .request
        .expected_revision
        .checked_add(1)
        .ok_or_else(|| WorkRunStoreError::Storage("WorkRun revision overflow".into()))?;
    if commit.result.work_run_id != commit.request.work_run_id
        || commit.result.revision != next_revision
    {
        return Err(WorkRunStoreError::Storage(
            "WorkRun commit does not contain the requested next aggregate revision".into(),
        ));
    }
    validate_run(&commit.result)?;
    Ok(())
}

fn serialize(value: &impl serde::Serialize) -> Result<String, WorkRunStoreError> {
    serde_json::to_string(value).map_err(|error| WorkRunStoreError::Storage(error.to_string()))
}

fn deserialize_run(
    work_run_id: &WorkRunId,
    revision: i64,
    record: &str,
) -> Result<WorkRun, WorkRunStoreError> {
    let run = serde_json::from_str::<WorkRun>(record)
        .map_err(|error| WorkRunStoreError::Storage(error.to_string()))?;
    let revision = from_sql_integer(revision).map_err(WorkRunStoreError::Storage)?;
    if &run.work_run_id != work_run_id || run.revision != revision {
        return Err(WorkRunStoreError::Storage(
            "WorkRun row metadata disagrees with its record".into(),
        ));
    }
    validate_run(&run)?;
    Ok(run)
}

fn validate_run(run: &WorkRun) -> Result<(), WorkRunStoreError> {
    run.validate()
        .map_err(|error| WorkRunStoreError::Storage(format!("invalid WorkRun record: {error}")))
}

fn storage_error(error: impl std::fmt::Display) -> WorkRunStoreError {
    WorkRunStoreError::Storage(format!("WorkRun SQLite error: {error}"))
}
