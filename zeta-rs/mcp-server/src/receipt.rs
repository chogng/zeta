use crate::agent::{AgentCallError, AgentOutcome, InvocationFingerprint};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;
use zeta_protocol::{SessionId, ThreadId};

const RECEIPT_SQLITE_SCHEMA_VERSION: u32 = 1;

pub(crate) enum BeginInvocation {
    Execute,
    Replay(AgentOutcome),
}

pub(crate) struct ReceiptStore {
    inner: Mutex<ReceiptState>,
}

struct ReceiptState {
    connection: Connection,
    active: BTreeSet<(String, String)>,
}

impl ReceiptStore {
    pub(crate) fn open(path: &Path) -> Result<Self, AgentCallError> {
        let mut connection =
            zeta_state::open_sqlite_database(path, zeta_state::SqliteDurability::Durable)
                .map_err(receipt_error)?;
        prepare_schema_registry(&connection)?;
        initialize_schema(&mut connection)?;
        Ok(Self::from_connection(connection))
    }

    #[cfg(test)]
    pub(crate) fn memory() -> Self {
        let mut connection =
            zeta_state::open_in_memory_database(zeta_state::SqliteDurability::Durable)
                .expect("open in-memory receipt database");
        prepare_schema_registry(&connection).expect("prepare receipt schema registry");
        initialize_schema(&mut connection).expect("initialize in-memory receipt schema");
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Self {
        Self {
            inner: Mutex::new(ReceiptState {
                connection,
                active: BTreeSet::new(),
            }),
        }
    }

    pub(crate) fn begin(
        &self,
        principal: &str,
        invocation_id: &str,
        fingerprint: InvocationFingerprint,
    ) -> Result<BeginInvocation, AgentCallError> {
        let key = (principal.to_string(), invocation_id.to_string());
        let mut state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        if state.active.contains(&key) {
            return Err(AgentCallError::InvocationInProgress);
        }
        let fingerprint = fingerprint.encode();
        let transaction = state
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(receipt_error)?;
        let existing = transaction
            .query_row(
                "SELECT fingerprint, state, outcome_json
                 FROM mcp_invocation_receipts
                 WHERE principal = ?1 AND invocation_id = ?2",
                params![principal, invocation_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(receipt_error)?;
        let disposition = match existing {
            Some((stored_fingerprint, _, _)) if stored_fingerprint != fingerprint => {
                return Err(AgentCallError::InvocationConflict);
            }
            Some((_, state, outcome)) if state == "finished" => {
                let outcome = outcome.ok_or_else(|| {
                    AgentCallError::AppServer(
                        "finished MCP invocation receipt has no outcome".into(),
                    )
                })?;
                BeginInvocation::Replay(serde_json::from_str(&outcome).map_err(receipt_error)?)
            }
            Some((_, state, _)) if state == "running" => BeginInvocation::Execute,
            Some((_, state, _)) => {
                return Err(AgentCallError::AppServer(format!(
                    "unsupported MCP invocation receipt state '{state}'"
                )));
            }
            None => {
                transaction
                    .execute(
                        "INSERT INTO mcp_invocation_receipts
                         (principal, invocation_id, fingerprint, state, outcome_json)
                         VALUES (?1, ?2, ?3, 'running', NULL)",
                        params![principal, invocation_id, fingerprint],
                    )
                    .map_err(receipt_error)?;
                BeginInvocation::Execute
            }
        };
        transaction.commit().map_err(receipt_error)?;
        if matches!(disposition, BeginInvocation::Execute) {
            state.active.insert(key);
        }
        Ok(disposition)
    }

    pub(crate) fn finish(
        &self,
        principal: &str,
        invocation_id: &str,
        fingerprint: InvocationFingerprint,
        result: Result<AgentOutcome, AgentCallError>,
    ) -> Result<AgentOutcome, AgentCallError> {
        let mut state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        state
            .active
            .remove(&(principal.to_string(), invocation_id.to_string()));
        let transaction = state
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(receipt_error)?;
        match &result {
            Ok(outcome) if outcome.is_terminal() => {
                transaction
                    .execute(
                        "INSERT INTO mcp_invocation_receipts
                         (principal, invocation_id, fingerprint, state, outcome_json)
                         VALUES (?1, ?2, ?3, 'finished', ?4)
                         ON CONFLICT(principal, invocation_id) DO UPDATE SET
                           fingerprint = excluded.fingerprint,
                           state = excluded.state,
                           outcome_json = excluded.outcome_json",
                        params![
                            principal,
                            invocation_id,
                            fingerprint.encode(),
                            serde_json::to_string(outcome).map_err(receipt_error)?,
                        ],
                    )
                    .map_err(receipt_error)?;
            }
            Ok(_) => {}
            Err(_) => {
                transaction
                    .execute(
                        "DELETE FROM mcp_invocation_receipts
                         WHERE principal = ?1 AND invocation_id = ?2",
                        params![principal, invocation_id],
                    )
                    .map_err(receipt_error)?;
            }
        }
        transaction.commit().map_err(receipt_error)?;
        result
    }

    pub(crate) fn bind_thread(
        &self,
        principal: &str,
        thread_id: ThreadId,
        session_id: SessionId,
    ) -> Result<(), AgentCallError> {
        let mut state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        let transaction = state
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(receipt_error)?;
        let existing = transaction
            .query_row(
                "SELECT session_id FROM mcp_thread_bindings
                 WHERE principal = ?1 AND thread_id = ?2",
                params![principal, thread_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(receipt_error)?;
        match existing {
            Some(existing) if existing != session_id.as_str() => {
                return Err(AgentCallError::AppServer(
                    "durable Thread binding conflicts with the App Server result".into(),
                ));
            }
            Some(_) => {}
            None => {
                transaction
                    .execute(
                        "INSERT INTO mcp_thread_bindings
                         (principal, thread_id, session_id) VALUES (?1, ?2, ?3)",
                        params![principal, thread_id.as_str(), session_id.as_str()],
                    )
                    .map_err(receipt_error)?;
            }
        }
        transaction.commit().map_err(receipt_error)
    }

    pub(crate) fn session_for_thread(
        &self,
        principal: &str,
        thread_id: &ThreadId,
    ) -> Result<Option<SessionId>, AgentCallError> {
        let state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        state
            .connection
            .query_row(
                "SELECT session_id FROM mcp_thread_bindings
                 WHERE principal = ?1 AND thread_id = ?2",
                params![principal, thread_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(receipt_error)?
            .map(|session_id| SessionId::new(session_id).map_err(receipt_error))
            .transpose()
    }
}

fn prepare_schema_registry(connection: &Connection) -> Result<(), AgentCallError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS zeta_schema_migrations (
                 component TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             );",
        )
        .map_err(receipt_error)
}

fn initialize_schema(connection: &mut Connection) -> Result<(), AgentCallError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(receipt_error)?;
    let version = transaction
        .query_row(
            "SELECT version FROM zeta_schema_migrations WHERE component = 'mcp-receipts'",
            [],
            |row| row.get::<_, u32>(0),
        )
        .optional()
        .map_err(receipt_error)?;
    match version {
        Some(version) if version != RECEIPT_SQLITE_SCHEMA_VERSION => {
            return Err(AgentCallError::AppServer(format!(
                "unsupported MCP receipt SQLite schema version {version}"
            )));
        }
        Some(_) => {}
        None => {
            transaction
                .execute_batch(
                    "CREATE TABLE mcp_invocation_receipts (
                         principal TEXT NOT NULL,
                         invocation_id TEXT NOT NULL,
                         fingerprint TEXT NOT NULL,
                         state TEXT NOT NULL CHECK (state IN ('running', 'finished')),
                         outcome_json TEXT,
                         PRIMARY KEY (principal, invocation_id),
                         CHECK (
                           (state = 'running' AND outcome_json IS NULL) OR
                           (state = 'finished' AND outcome_json IS NOT NULL)
                         )
                     );
                     CREATE TABLE mcp_thread_bindings (
                         principal TEXT NOT NULL,
                         thread_id TEXT NOT NULL,
                         session_id TEXT NOT NULL,
                         PRIMARY KEY (principal, thread_id)
                     );",
                )
                .map_err(receipt_error)?;
            transaction
                .execute(
                    "INSERT INTO zeta_schema_migrations (component, version)
                     VALUES ('mcp-receipts', ?1)",
                    [RECEIPT_SQLITE_SCHEMA_VERSION],
                )
                .map_err(receipt_error)?;
        }
    }
    transaction.commit().map_err(receipt_error)
}

fn receipt_lock_error() -> AgentCallError {
    AgentCallError::AppServer("MCP receipt lock poisoned".into())
}

fn receipt_error(error: impl std::fmt::Display) -> AgentCallError {
    AgentCallError::AppServer(format!("MCP SQLite receipt store failed: {error}"))
}

#[cfg(test)]
#[path = "receipt_tests.rs"]
mod tests;
