use crate::DocumentCollaborationOpenParams;
use crate::DocumentCollaborationOpenResult;
use crate::DocumentCollaborationSubmitParams;
use crate::DocumentCollaborationSubmitResult;
use crate::DocumentCollaborationUpdate;
use crate::room::DocumentCollaborationReplay;
use crate::room::MAX_JAVASCRIPT_SAFE_INTEGER;
use crate::room::MAX_ROOM_HISTORY;
use crate::room::random_room_id;
use crate::room::replay;
use crate::room::replay_submit_result;
use crate::room::snapshot;
use crate::room::validate_document;
use crate::room::validate_identifier;
use crate::room::validate_javascript_safe_integer;
use crate::room::validate_transaction;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Transaction;
use rusqlite::TransactionBehavior;
use rusqlite::params;
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed collaboration authority that survives remote-host restarts.
pub struct SqliteDocumentCollaborationRooms {
    connection: Mutex<Connection>,
}

struct PersistedRoom {
    schema_id: String,
    document: String,
    version: u64,
}

impl SqliteDocumentCollaborationRooms {
    /// Opens or creates the collaboration authority at an explicit SQLite path.
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self, String> {
        let mut connection = Connection::open(path).map_err(sqlite_error)?;
        configure(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn open(
        &self,
        params: DocumentCollaborationOpenParams,
    ) -> Result<DocumentCollaborationOpenResult, String> {
        validate_identifier(&params.client_id, "clientId")?;
        validate_identifier(&params.schema_id, "schemaId")?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite_error)?;
        let room_id = match params.room_id {
            Some(room_id) => {
                validate_identifier(&room_id, "roomId")?;
                room_id
            }
            None => create_room_id(&transaction)?,
        };
        if let Some(room) = load_room(&transaction, &room_id)? {
            if room.schema_id != params.schema_id {
                return Err("The collaboration room uses a different document schema".into());
            }
            let result = DocumentCollaborationOpenResult {
                client_id: params.client_id,
                schema_id: room.schema_id,
                snapshot: snapshot(&room_id, room.version, room.document),
            };
            transaction.commit().map_err(sqlite_error)?;
            return Ok(result);
        }
        validate_document(&params.document)?;
        transaction
            .execute(
                "INSERT INTO document_collaboration_rooms (room_id, schema_id, document, version) VALUES (?1, ?2, ?3, 0)",
                params![room_id, params.schema_id, params.document],
            )
            .map_err(sqlite_error)?;
        let result = DocumentCollaborationOpenResult {
            client_id: params.client_id,
            schema_id: params.schema_id,
            snapshot: snapshot(&room_id, 0, params.document),
        };
        transaction.commit().map_err(sqlite_error)?;
        Ok(result)
    }

    pub fn submit(
        &self,
        params: DocumentCollaborationSubmitParams,
    ) -> Result<DocumentCollaborationSubmitResult, String> {
        validate_identifier(&params.room_id, "roomId")?;
        validate_identifier(&params.client_id, "clientId")?;
        validate_javascript_safe_integer(params.sequence, "sequence", 1)?;
        validate_javascript_safe_integer(params.base_version, "baseVersion", 0)?;
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite_error)?;
        let Some(room) = load_room(&transaction, &params.room_id)? else {
            return Err("The collaboration room does not exist".into());
        };
        if let Some(existing) = load_client_update(
            &transaction,
            &params.room_id,
            &params.client_id,
            params.sequence,
        )? {
            if existing.update.base_version != params.base_version
                || existing.update.transaction != params.transaction
                || existing.document != params.document
            {
                return Err("sequence has already been used by this collaboration client".into());
            }
            transaction.commit().map_err(sqlite_error)?;
            return Ok(DocumentCollaborationSubmitResult::Accepted {
                update: existing.update,
            });
        }
        if params.base_version > room.version {
            return Err("baseVersion cannot be newer than the collaboration room".into());
        }
        if params.base_version != room.version {
            let replay = replay_room(&transaction, &params.room_id, room, params.base_version)?;
            transaction.commit().map_err(sqlite_error)?;
            return Ok(replay_submit_result(replay));
        }
        validate_transaction(&params.transaction)?;
        validate_document(&params.document)?;
        let version = room
            .version
            .checked_add(1)
            .filter(|version| *version <= MAX_JAVASCRIPT_SAFE_INTEGER)
            .ok_or_else(|| {
                "The collaboration room version exceeded JavaScript's safe integer range"
                    .to_string()
            })?;
        let update = DocumentCollaborationUpdate {
            room_id: params.room_id,
            client_id: params.client_id,
            sequence: params.sequence,
            base_version: params.base_version,
            version,
            transaction: params.transaction,
        };
        transaction
            .execute(
                "UPDATE document_collaboration_rooms SET document = ?2, version = ?3 WHERE room_id = ?1",
                params![&update.room_id, &params.document, to_sql_integer(version)?],
            )
            .map_err(sqlite_error)?;
        transaction
            .execute(
                "INSERT INTO document_collaboration_updates (room_id, version, client_id, sequence, base_version, transaction_json, document) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &update.room_id,
                    to_sql_integer(update.version)?,
                    &update.client_id,
                    to_sql_integer(update.sequence)?,
                    to_sql_integer(update.base_version)?,
                    &update.transaction,
                    &params.document,
                ],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(DocumentCollaborationSubmitResult::Accepted { update })
    }

    pub fn replay(
        &self,
        room_id: &str,
        after_version: u64,
    ) -> Result<DocumentCollaborationReplay, String> {
        validate_identifier(room_id, "roomId")?;
        validate_javascript_safe_integer(after_version, "afterVersion", 0)?;
        let connection = self.connection.lock().map_err(lock_error)?;
        let Some(room) = load_room(&connection, room_id)? else {
            return Err("The collaboration room does not exist".into());
        };
        if after_version > room.version {
            return Err("afterVersion cannot be newer than the collaboration room".into());
        }
        replay_room(&connection, room_id, room, after_version)
    }
}

fn configure(connection: &mut Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS document_collaboration_rooms (
                room_id TEXT PRIMARY KEY NOT NULL,
                schema_id TEXT NOT NULL,
                document TEXT NOT NULL,
                version INTEGER NOT NULL CHECK(version >= 0)
            );
            CREATE TABLE IF NOT EXISTS document_collaboration_updates (
                room_id TEXT NOT NULL REFERENCES document_collaboration_rooms(room_id) ON DELETE CASCADE,
                version INTEGER NOT NULL CHECK(version > 0),
                client_id TEXT NOT NULL,
                sequence INTEGER NOT NULL CHECK(sequence > 0),
                base_version INTEGER NOT NULL CHECK(base_version >= 0),
                transaction_json TEXT NOT NULL,
                document TEXT NOT NULL,
                PRIMARY KEY (room_id, version),
                UNIQUE (room_id, client_id, sequence)
            );
            CREATE INDEX IF NOT EXISTS document_collaboration_updates_replay
                ON document_collaboration_updates (room_id, version);
            ",
        )
        .map_err(sqlite_error)
}

fn create_room_id(transaction: &Transaction<'_>) -> Result<String, String> {
    loop {
        let room_id = random_room_id()?;
        if load_room(transaction, &room_id)?.is_none() {
            return Ok(room_id);
        }
    }
}

fn load_room(connection: &Connection, room_id: &str) -> Result<Option<PersistedRoom>, String> {
    connection
        .query_row(
            "SELECT schema_id, document, version FROM document_collaboration_rooms WHERE room_id = ?1",
            [room_id],
            |row| {
                Ok(PersistedRoom {
                    schema_id: row.get(0)?,
                    document: row.get(1)?,
                    version: from_sql_integer(row.get(2)?)?,
                })
            },
        )
        .optional()
        .map_err(sqlite_error)
}

struct PersistedUpdate {
    update: DocumentCollaborationUpdate,
    document: String,
}

fn load_client_update(
    connection: &Connection,
    room_id: &str,
    client_id: &str,
    sequence: u64,
) -> Result<Option<PersistedUpdate>, String> {
    connection
        .query_row(
            "SELECT version, base_version, transaction_json, document FROM document_collaboration_updates WHERE room_id = ?1 AND client_id = ?2 AND sequence = ?3",
            params![room_id, client_id, to_sql_integer(sequence)?],
            |row| {
                Ok(PersistedUpdate {
                    update: DocumentCollaborationUpdate {
                        room_id: room_id.into(),
                        client_id: client_id.into(),
                        sequence,
                        base_version: from_sql_integer(row.get(1)?)?,
                        version: from_sql_integer(row.get(0)?)?,
                        transaction: row.get(2)?,
                    },
                    document: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(sqlite_error)
}

fn replay_room(
    connection: &Connection,
    room_id: &str,
    room: PersistedRoom,
    after_version: u64,
) -> Result<DocumentCollaborationReplay, String> {
    let updates = load_updates(connection, room_id)?;
    Ok(replay(
        room_id,
        room.version,
        room.document,
        updates,
        after_version,
    ))
}

fn load_updates(
    connection: &Connection,
    room_id: &str,
) -> Result<Vec<DocumentCollaborationUpdate>, String> {
    let mut statement = connection
        .prepare(
            "SELECT version, client_id, sequence, base_version, transaction_json FROM document_collaboration_updates WHERE room_id = ?1 ORDER BY version DESC LIMIT ?2",
        )
        .map_err(sqlite_error)?;
    let rows = statement
        .query_map(params![room_id, MAX_ROOM_HISTORY as i64], |row| {
            Ok(DocumentCollaborationUpdate {
                room_id: room_id.into(),
                version: from_sql_integer(row.get(0)?)?,
                client_id: row.get(1)?,
                sequence: from_sql_integer(row.get(2)?)?,
                base_version: from_sql_integer(row.get(3)?)?,
                transaction: row.get(4)?,
            })
        })
        .map_err(sqlite_error)?;
    let mut updates = rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_error)?;
    updates.reverse();
    Ok(updates)
}

fn to_sql_integer(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| "Collaboration version exceeded SQLite's integer range".into())
}

fn from_sql_integer(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, value))
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> String {
    "Collaboration database lock poisoned".into()
}

fn sqlite_error(error: rusqlite::Error) -> String {
    format!("Collaboration database error: {error}")
}
