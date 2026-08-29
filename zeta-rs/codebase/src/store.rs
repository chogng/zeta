use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use rusqlite::OptionalExtension;
use rusqlite::Transaction;
use rusqlite::params;

use crate::ChunkContentHash;
use crate::ChunkKey;
use crate::ChunkReference;
use crate::ChunkSpan;
use crate::CodebaseError;
use crate::CodebaseSnapshot;
use crate::CodebaseStorage;
use crate::IndexRootId;
use crate::IndexedLanguage;
use crate::SearchHit;
use crate::SourceRevision;
use crate::chunker::CHUNKER_VERSION;
use crate::scanner::PreparedFile;
use crate::scanner::WorkspaceScan;
use crate::store_manifest::load_manifest;
use crate::types::CodebaseManifest;

const SCHEMA_VERSION: &str = "2";

pub(crate) enum FileUpdate {
    Remove(PathBuf),
    Upsert(PreparedFile),
}

pub(crate) struct StoredSource {
    pub revision: SourceRevision,
    pub source_bytes: usize,
}

pub(crate) struct IndexStore {
    connection: Mutex<Connection>,
}

impl IndexStore {
    pub fn open(storage: &CodebaseStorage, root_id: &IndexRootId) -> Result<Self, CodebaseError> {
        let connection = match storage {
            CodebaseStorage::Memory => Connection::open_in_memory()?,
            CodebaseStorage::Persistent(path) => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|source| CodebaseError::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
                prepare_persistent_database(path)?;
                Connection::open(path)?
            }
        };
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        if matches!(storage, CodebaseStorage::Persistent(_)) {
            connection.pragma_update(None, "journal_mode", "WAL")?;
            connection.pragma_update(None, "synchronous", "NORMAL")?;
        }
        let store = Self {
            connection: Mutex::new(connection),
        };
        store.initialize(root_id)?;
        Ok(store)
    }

    pub fn replace_workspace(
        &self,
        root_id: &IndexRootId,
        scan: WorkspaceScan,
    ) -> Result<CodebaseSnapshot, CodebaseError> {
        let mut connection = self.connection.lock().expect("codebase store poisoned");
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM codebase_chunks_fts", [])?;
        transaction.execute("DELETE FROM codebase_chunks", [])?;
        transaction.execute("DELETE FROM codebase_files", [])?;
        for file in &scan.files {
            insert_file(&transaction, file)?;
        }
        let generation = increment_generation(&transaction)?;
        set_metadata(
            &transaction,
            "skipped_file_count",
            &scan.skipped_file_count.to_string(),
        )?;
        set_metadata(
            &transaction,
            "file_limit_hit",
            flag_value(scan.file_limit_hit),
        )?;
        set_metadata(
            &transaction,
            "source_bytes_limit_hit",
            flag_value(scan.source_bytes_limit_hit),
        )?;
        let snapshot = snapshot_from_connection(&transaction, root_id, generation)?;
        transaction.commit()?;
        Ok(snapshot)
    }

    pub fn publish_updates(
        &self,
        root_id: &IndexRootId,
        updates: Vec<FileUpdate>,
    ) -> Result<CodebaseSnapshot, CodebaseError> {
        let mut connection = self.connection.lock().expect("codebase store poisoned");
        let transaction = connection.transaction()?;
        for update in updates {
            match update {
                FileUpdate::Remove(path) => remove_file(&transaction, &path)?,
                FileUpdate::Upsert(file) => {
                    remove_file(&transaction, &file.relative_path)?;
                    insert_file(&transaction, &file)?;
                }
            }
        }
        let generation = increment_generation(&transaction)?;
        let snapshot = snapshot_from_connection(&transaction, root_id, generation)?;
        transaction.commit()?;
        Ok(snapshot)
    }

    pub fn snapshot(&self, root_id: &IndexRootId) -> Result<CodebaseSnapshot, CodebaseError> {
        let connection = self.connection.lock().expect("codebase store poisoned");
        let generation = metadata(&connection, "generation")?
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        snapshot_from_connection(&connection, root_id, generation)
    }

    pub fn manifest(&self, root_id: &IndexRootId) -> Result<CodebaseManifest, CodebaseError> {
        let connection = self.connection.lock().expect("codebase store poisoned");
        load_manifest(&connection, root_id)
    }

    pub fn source(&self, relative_path: &Path) -> Result<Option<StoredSource>, CodebaseError> {
        let connection = self.connection.lock().expect("codebase store poisoned");
        let path = storage_path(relative_path);
        connection
            .query_row(
                "SELECT source_revision, source_bytes FROM codebase_files WHERE path = ?1",
                [path],
                |row| {
                    Ok(StoredSource {
                        revision: SourceRevision::new(row.get(0)?),
                        source_bytes: to_usize(row.get::<_, i64>(1)?),
                    })
                },
            )
            .optional()
            .map_err(CodebaseError::from)
    }

    pub fn has_descendants(&self, relative_path: &Path) -> Result<bool, CodebaseError> {
        let connection = self.connection.lock().expect("codebase store poisoned");
        let prefix = relative_path.to_path_buf();
        let mut statement = connection.prepare("SELECT path FROM codebase_files")?;
        let paths = statement.query_map([], |row| row.get::<_, String>(0))?;
        for path in paths {
            if PathBuf::from(path?).starts_with(&prefix) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn search(
        &self,
        root_id: &IndexRootId,
        expression: &str,
        result_limit: usize,
    ) -> Result<Vec<SearchHit>, CodebaseError> {
        let connection = self.connection.lock().expect("codebase store poisoned");
        let mut statement = connection.prepare(
            "SELECT c.path, c.source_revision, c.chunk_key, c.content_hash, c.language, \
                    c.start_byte, c.end_byte, c.start_line, c.end_line_exclusive, c.content, \
                    bm25(codebase_chunks_fts) \
             FROM codebase_chunks_fts \
             JOIN codebase_chunks c ON c.rowid = codebase_chunks_fts.chunk_rowid \
             WHERE codebase_chunks_fts MATCH ?1 \
             ORDER BY bm25(codebase_chunks_fts), c.path, c.ordinal \
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![expression, to_i64(result_limit)], |row| {
            let rank = row.get::<_, f64>(10)?;
            Ok(SearchHit {
                reference: ChunkReference {
                    root_id: root_id.clone(),
                    relative_path: PathBuf::from(row.get::<_, String>(0)?),
                    source_revision: SourceRevision::new(row.get(1)?),
                    key: ChunkKey::new(row.get(2)?),
                    content_hash: ChunkContentHash::new(row.get(3)?),
                    span: ChunkSpan {
                        start_byte: to_usize(row.get::<_, i64>(5)?),
                        end_byte: to_usize(row.get::<_, i64>(6)?),
                        start_line: to_usize(row.get::<_, i64>(7)?),
                        end_line_exclusive: to_usize(row.get::<_, i64>(8)?),
                    },
                },
                language: IndexedLanguage::from_id(&row.get::<_, String>(4)?),
                content: row.get(9)?,
                score: -rank,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CodebaseError::from)
    }

    fn initialize(&self, root_id: &IndexRootId) -> Result<(), CodebaseError> {
        let mut connection = self.connection.lock().expect("codebase store poisoned");
        create_schema(&connection)?;
        let stored_root = metadata(&connection, "root_id")?;
        if stored_root
            .as_deref()
            .is_some_and(|stored| stored != root_id.as_str())
        {
            return Err(CodebaseError::StorageRootMismatch);
        }
        let stored_schema = metadata(&connection, "schema_version")?;
        let stored_chunker = metadata(&connection, "chunker_version")?;
        if stored_schema
            .as_deref()
            .is_some_and(|stored| stored != SCHEMA_VERSION)
            || stored_chunker
                .as_deref()
                .is_some_and(|stored| stored != CHUNKER_VERSION)
        {
            reset_projection(&mut connection)?;
        }
        let transaction = connection.transaction()?;
        set_metadata(&transaction, "root_id", root_id.as_str())?;
        set_metadata(&transaction, "schema_version", SCHEMA_VERSION)?;
        set_metadata(&transaction, "chunker_version", CHUNKER_VERSION)?;
        if metadata(&transaction, "generation")?.is_none() {
            set_metadata(&transaction, "generation", "0")?;
        }
        transaction.commit()?;
        Ok(())
    }
}

fn create_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS codebase_metadata (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS codebase_files (
             path TEXT PRIMARY KEY,
             source_revision TEXT NOT NULL,
             language TEXT NOT NULL,
             source_bytes INTEGER NOT NULL,
             chunk_count INTEGER NOT NULL,
             chunk_limit_hit INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS codebase_chunks (
             rowid INTEGER PRIMARY KEY,
             path TEXT NOT NULL REFERENCES codebase_files(path) ON DELETE CASCADE,
             ordinal INTEGER NOT NULL,
             source_revision TEXT NOT NULL,
             chunk_key TEXT NOT NULL,
             content_hash TEXT NOT NULL,
             language TEXT NOT NULL,
             start_byte INTEGER NOT NULL,
             end_byte INTEGER NOT NULL,
             start_line INTEGER NOT NULL,
             end_line_exclusive INTEGER NOT NULL,
             content TEXT NOT NULL,
             UNIQUE(path, ordinal)
         );
         CREATE INDEX IF NOT EXISTS codebase_chunks_key ON codebase_chunks(chunk_key);
         CREATE VIRTUAL TABLE IF NOT EXISTS codebase_chunks_fts USING fts5(
             path,
             content,
             chunk_rowid UNINDEXED,
             tokenize = 'unicode61'
         );",
    )
}

#[cfg(unix)]
fn prepare_persistent_database(path: &Path) -> Result<(), CodebaseError> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::fs::PermissionsExt;

    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(CodebaseError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "codebase database must be a regular file",
                ),
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(CodebaseError::Io {
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
        .map_err(|source| CodebaseError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        CodebaseError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn prepare_persistent_database(_path: &Path) -> Result<(), CodebaseError> {
    Ok(())
}

fn reset_projection(connection: &mut Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "DROP TABLE IF EXISTS codebase_chunks_fts;
         DROP TABLE IF EXISTS codebase_chunks;
         DROP TABLE IF EXISTS codebase_files;
         DROP TABLE IF EXISTS codebase_metadata;",
    )?;
    create_schema(connection)
}

fn insert_file(transaction: &Transaction<'_>, file: &PreparedFile) -> rusqlite::Result<()> {
    let path = storage_path(&file.relative_path);
    transaction.execute(
        "INSERT INTO codebase_files(
             path, source_revision, language, source_bytes, chunk_count, chunk_limit_hit
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            path,
            file.source_revision.as_str(),
            file.language.id(),
            to_i64(file.source_bytes),
            to_i64(file.chunks.len()),
            flag_value(file.chunk_limit_hit),
        ],
    )?;
    for (ordinal, chunk) in file.chunks.iter().enumerate() {
        transaction.execute(
            "INSERT INTO codebase_chunks(
                 path, ordinal, source_revision, chunk_key, content_hash, language,
                 start_byte, end_byte, start_line, end_line_exclusive, content
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                path,
                to_i64(ordinal),
                file.source_revision.as_str(),
                chunk.key.as_str(),
                chunk.content_hash.as_str(),
                file.language.id(),
                to_i64(chunk.span.start_byte),
                to_i64(chunk.span.end_byte),
                to_i64(chunk.span.start_line),
                to_i64(chunk.span.end_line_exclusive),
                chunk.content,
            ],
        )?;
        let rowid = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO codebase_chunks_fts(path, content, chunk_rowid) VALUES (?1, ?2, ?3)",
            params![path, chunk.content, rowid],
        )?;
    }
    Ok(())
}

fn remove_file(transaction: &Transaction<'_>, relative_path: &Path) -> rusqlite::Result<()> {
    let path = storage_path(relative_path);
    transaction.execute(
        "DELETE FROM codebase_chunks_fts WHERE chunk_rowid IN (
             SELECT rowid FROM codebase_chunks WHERE path = ?1
         )",
        [&path],
    )?;
    transaction.execute("DELETE FROM codebase_chunks WHERE path = ?1", [&path])?;
    transaction.execute("DELETE FROM codebase_files WHERE path = ?1", [&path])?;
    Ok(())
}

fn snapshot_from_connection(
    connection: &Connection,
    root_id: &IndexRootId,
    generation: u64,
) -> Result<CodebaseSnapshot, CodebaseError> {
    let (file_count, source_bytes) = connection.query_row(
        "SELECT COUNT(*), COALESCE(SUM(source_bytes), 0) FROM codebase_files",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )?;
    let chunk_count = connection.query_row("SELECT COUNT(*) FROM codebase_chunks", [], |row| {
        row.get::<_, i64>(0)
    })?;
    let truncated_file_count = connection.query_row(
        "SELECT COUNT(*) FROM codebase_files WHERE chunk_limit_hit = 1",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(CodebaseSnapshot {
        root_id: root_id.clone(),
        generation,
        indexed_file_count: to_usize(file_count),
        indexed_chunk_count: to_usize(chunk_count),
        indexed_source_bytes: to_usize(source_bytes),
        skipped_file_count: metadata(connection, "skipped_file_count")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        truncated_file_count: to_usize(truncated_file_count),
        file_limit_hit: metadata_flag(connection, "file_limit_hit")?,
        source_bytes_limit_hit: metadata_flag(connection, "source_bytes_limit_hit")?,
    })
}

fn increment_generation(transaction: &Transaction<'_>) -> rusqlite::Result<u64> {
    let generation = metadata(transaction, "generation")?
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    set_metadata(transaction, "generation", &generation.to_string())?;
    Ok(generation)
}

fn metadata(connection: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM codebase_metadata WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
}

fn set_metadata(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO codebase_metadata(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn metadata_flag(connection: &Connection, key: &str) -> rusqlite::Result<bool> {
    metadata(connection, key).map(|value| value.as_deref() == Some("1"))
}

fn flag_value(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn storage_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
