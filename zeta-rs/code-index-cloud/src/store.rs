use std::fs;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::params;
use serde::Deserialize;
use serde::Serialize;

use crate::CloudCodeIndexDestination;
use crate::CloudCodeIndexError;
use crate::CloudCodeIndexGrant;
use crate::CloudCodeIndexGrantId;
use crate::CloudCodeIndexSelection;
use crate::CloudCodeIndexState;
use crate::CloudCodeIndexStorage;

const SCHEMA_VERSION: &str = "2";
const WORKSPACE_CHUNK_PROJECTION_SCHEMA_VERSION: &str = "1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DurableCloudState {
    pub phase: CloudCodeIndexState,
    pub grant: Option<CloudCodeIndexGrant>,
    pub synced_local_generation: Option<u64>,
    pub remote_generation: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum LegacyCloudCodeIndexMode {
    Projection,
    Managed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct LegacyCloudCodeIndexGrant {
    id: CloudCodeIndexGrantId,
    root_id: String,
    mode: LegacyCloudCodeIndexMode,
    destination: CloudCodeIndexDestination,
    selection: CloudCodeIndexSelection,
    max_egress_bytes: std::num::NonZeroU64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct LegacyDurableCloudState {
    phase: CloudCodeIndexState,
    grant: Option<LegacyCloudCodeIndexGrant>,
    synced_local_generation: Option<u64>,
    remote_generation: Option<String>,
}

impl Default for DurableCloudState {
    fn default() -> Self {
        Self {
            phase: CloudCodeIndexState::LocalOnly,
            grant: None,
            synced_local_generation: None,
            remote_generation: None,
        }
    }
}

pub(crate) struct CloudStateStore {
    connection: Mutex<Connection>,
}

impl CloudStateStore {
    pub fn open(
        storage: &CloudCodeIndexStorage,
        root_id: &str,
    ) -> Result<Self, CloudCodeIndexError> {
        let mut connection = match storage {
            CloudCodeIndexStorage::Memory => Connection::open_in_memory()?,
            CloudCodeIndexStorage::Persistent(path) => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|source| CloudCodeIndexError::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
                prepare_persistent_database(path)?;
                Connection::open(path)?
            }
        };
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS cloud_code_index_metadata (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )?;
        let stored_root = metadata(&connection, "root_id")?;
        if stored_root
            .as_deref()
            .is_some_and(|stored| stored != root_id)
        {
            return Err(CloudCodeIndexError::StorageRootMismatch);
        }
        let stored_schema = metadata(&connection, "schema_version")?;
        match stored_schema.as_deref() {
            None | Some(SCHEMA_VERSION) => {}
            Some(WORKSPACE_CHUNK_PROJECTION_SCHEMA_VERSION) => {
                migrate_workspace_chunk_projection_state(&mut connection)?;
            }
            Some(_) => return Err(CloudCodeIndexError::IncompatibleStorage),
        }
        set_metadata(&connection, "root_id", root_id)?;
        set_metadata(&connection, "schema_version", SCHEMA_VERSION)?;
        if metadata(&connection, "state")?.is_none() {
            set_metadata(
                &connection,
                "state",
                &serde_json::to_string(&DurableCloudState::default())?,
            )?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn load(&self) -> Result<DurableCloudState, CloudCodeIndexError> {
        let connection = self.connection.lock().expect("cloud index store poisoned");
        let state =
            metadata(&connection, "state")?.ok_or(CloudCodeIndexError::IncompatibleStorage)?;
        Ok(serde_json::from_str(&state)?)
    }

    pub fn save(&self, state: &DurableCloudState) -> Result<(), CloudCodeIndexError> {
        let connection = self.connection.lock().expect("cloud index store poisoned");
        set_metadata(&connection, "state", &serde_json::to_string(state)?)?;
        Ok(())
    }
}

fn migrate_workspace_chunk_projection_state(
    connection: &mut Connection,
) -> Result<(), CloudCodeIndexError> {
    let state = metadata(connection, "state")?.ok_or(CloudCodeIndexError::IncompatibleStorage)?;
    let current = match serde_json::from_str::<LegacyDurableCloudState>(&state) {
        Ok(legacy) => {
            let managed_grant = legacy
                .grant
                .as_ref()
                .is_some_and(|grant| grant.mode == LegacyCloudCodeIndexMode::Managed);
            DurableCloudState {
                phase: if managed_grant {
                    CloudCodeIndexState::Revoking
                } else {
                    legacy.phase
                },
                grant: legacy.grant.map(|grant| CloudCodeIndexGrant {
                    id: grant.id,
                    root_id: grant.root_id,
                    destination: grant.destination,
                    selection: grant.selection,
                    max_egress_bytes: grant.max_egress_bytes,
                }),
                synced_local_generation: legacy.synced_local_generation,
                remote_generation: legacy.remote_generation,
            }
        }
        Err(_) => serde_json::from_str::<DurableCloudState>(&state)?,
    };
    let transaction = connection.transaction()?;
    set_metadata(&transaction, "state", &serde_json::to_string(&current)?)?;
    set_metadata(&transaction, "schema_version", SCHEMA_VERSION)?;
    transaction.commit()?;
    Ok(())
}

fn metadata(connection: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM cloud_code_index_metadata WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
}

fn set_metadata(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO cloud_code_index_metadata(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(unix)]
fn prepare_persistent_database(path: &Path) -> Result<(), CloudCodeIndexError> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::fs::PermissionsExt;

    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(CloudCodeIndexError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "cloud code-index database must be a regular file",
                ),
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(CloudCodeIndexError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    }
    fs::OpenOptions::new()
        .create(true)
        .read(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|source| CloudCodeIndexError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        CloudCodeIndexError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn prepare_persistent_database(_path: &Path) -> Result<(), CloudCodeIndexError> {
    Ok(())
}
