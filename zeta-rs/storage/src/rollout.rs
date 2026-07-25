use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use zeta_core::{CoreError, EventJournal};
use zeta_protocol::{AgentEvent, EventId, ThreadId, Timestamp};

/// Append-only durable history for all events belonging to one Thread state root.
pub struct RolloutLog {
    path: PathBuf,
    write_lock: Mutex<()>,
}

/// Maps each Thread to its own append-only rollout file under a single state root.
///
/// This is the production `EventJournal` adapter: every Thread owns an independent sequence and
/// no file carries events for a different Thread.
pub struct ThreadRolloutStore {
    root: PathBuf,
}

impl ThreadRolloutStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, CoreError> {
        let root = root.into();
        fs::create_dir_all(root.join("threads")).map_err(io_error)?;
        Ok(Self { root })
    }

    pub fn read_thread(&self, thread_id: &ThreadId) -> Result<Vec<AgentEvent>, CoreError> {
        let path = self.thread_path(thread_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        RolloutLog::open(path)?.read_all()
    }

    /// Reads every non-empty Thread rollout for Thread manager startup recovery.
    pub fn all_thread_events(&self) -> Result<Vec<Vec<AgentEvent>>, CoreError> {
        let mut rollouts = Vec::new();
        for entry in fs::read_dir(self.root.join("threads")).map_err(io_error)? {
            let path = entry.map_err(io_error)?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "rollout")
            {
                let events = RolloutLog::open(path)?.read_all()?;
                if !events.is_empty() {
                    rollouts.push(events);
                }
            }
        }
        Ok(rollouts)
    }

    /// Rebuilds the SQLite projection from every authoritative Thread rollout file.
    pub fn rebuild_sqlite_projection(
        &self,
        database_path: impl AsRef<Path>,
    ) -> Result<(), CoreError> {
        let database_path = database_path.as_ref();
        if database_path.exists() {
            fs::remove_file(database_path).map_err(io_error)?;
        }
        run_sql(
            database_path,
            "CREATE TABLE events (event_id TEXT PRIMARY KEY, sequence INTEGER NOT NULL, thread_id TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, occurred_at INTEGER NOT NULL);",
        )?;
        for entry in fs::read_dir(self.root.join("threads")).map_err(io_error)? {
            let path = entry.map_err(io_error)?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "rollout")
            {
                for event in RolloutLog::open(path)?.read_all()? {
                    let sql = format!(
                        "INSERT INTO events (event_id, sequence, thread_id, kind, payload, occurred_at) VALUES ({}, {}, {}, {}, {}, {});",
                        quoted(&event.event_id.0),
                        event.sequence,
                        quoted(event.thread_id.as_str()),
                        quoted(&event.kind),
                        quoted(&event.payload),
                        event.occurred_at.0
                    );
                    run_sql(database_path, &sql)?;
                }
            }
        }
        Ok(())
    }

    fn thread_path(&self, thread_id: &ThreadId) -> PathBuf {
        self.root
            .join("threads")
            .join(format!("{}.rollout", encode_hex(thread_id.as_str())))
    }
}

impl RolloutLog {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, CoreError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(io_error)?;
        recover_incomplete_tail(&path)?;
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read_all(&self) -> Result<Vec<AgentEvent>, CoreError> {
        let file = OpenOptions::new()
            .read(true)
            .open(&self.path)
            .map_err(io_error)?;
        BufReader::new(file)
            .lines()
            .map(|line| parse_event(&line.map_err(io_error)?))
            .collect()
    }

    /// Recreates the SQLite query projection solely from durable rollout records.
    pub fn rebuild_sqlite_projection(
        &self,
        database_path: impl AsRef<Path>,
    ) -> Result<(), CoreError> {
        let events = self.read_all()?;
        let database_path = database_path.as_ref();
        if database_path.exists() {
            fs::remove_file(database_path).map_err(io_error)?;
        }
        let schema = "CREATE TABLE events (event_id TEXT PRIMARY KEY, sequence INTEGER NOT NULL, thread_id TEXT NOT NULL, kind TEXT NOT NULL, occurred_at INTEGER NOT NULL);";
        run_sql(database_path, schema)?;
        for event in events {
            let sql = format!(
                "INSERT INTO events (event_id, sequence, thread_id, kind, occurred_at) VALUES ({}, {}, {}, {}, {});",
                quoted(&event.event_id.0),
                event.sequence,
                quoted(event.thread_id.as_str()),
                quoted(&event.kind),
                event.occurred_at.0
            );
            run_sql(database_path, &sql)?;
        }
        Ok(())
    }
}

impl EventJournal for RolloutLog {
    fn append(&self, event: &AgentEvent) -> Result<(), CoreError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| CoreError::Journal("rollout write lock poisoned".into()))?;
        let existing = self.read_all()?;
        if existing
            .iter()
            .rev()
            .find(|existing| existing.thread_id == event.thread_id)
            .is_some_and(|last| last.sequence >= event.sequence)
        {
            return Err(CoreError::Journal(
                "rollout sequence must strictly increase".into(),
            ));
        }
        if existing
            .iter()
            .any(|existing| existing.event_id == event.event_id)
        {
            return Err(CoreError::Journal("rollout event id must be unique".into()));
        }
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(io_error)?;
        let record = format!(
            "1\t{}\t{}\t{}\t{}\t{}\t{}",
            event.sequence,
            encode_hex(&event.event_id.0),
            encode_hex(event.thread_id.as_str()),
            encode_hex(&event.kind),
            encode_hex(&event.payload),
            event.occurred_at.0,
        );
        writeln!(file, "{record}\t{}", checksum(&record)).map_err(io_error)?;
        file.sync_data().map_err(io_error)
    }
}

impl EventJournal for ThreadRolloutStore {
    fn append(&self, event: &AgentEvent) -> Result<(), CoreError> {
        RolloutLog::open(self.thread_path(&event.thread_id))?.append(event)
    }
}

fn parse_event(line: &str) -> Result<AgentEvent, CoreError> {
    let fields: Vec<_> = line.split('\t').collect();
    match fields.as_slice() {
        [
            schema_version,
            sequence,
            event_id,
            thread_id,
            kind,
            payload,
            recorded_at,
            record_checksum,
        ] if *schema_version == "1" => {
            let record = fields[..7].join("\t");
            if checksum(&record) != *record_checksum {
                return Err(CoreError::Journal("rollout checksum mismatch".into()));
            }
            Ok(AgentEvent {
                event_id: EventId(decode_hex(event_id)?),
                sequence: sequence
                    .parse()
                    .map_err(|_| CoreError::Journal("invalid rollout sequence".into()))?,
                thread_id: ThreadId::new(decode_hex(thread_id)?),
                kind: decode_hex(kind)?,
                payload: decode_hex(payload)?,
                occurred_at: Timestamp(
                    recorded_at
                        .parse()
                        .map_err(|_| CoreError::Journal("invalid rollout timestamp".into()))?,
                ),
            })
        }
        [event_id, sequence, thread_id, kind, recorded_at] => Ok(AgentEvent {
            event_id: EventId((*event_id).into()),
            sequence: sequence
                .parse()
                .map_err(|_| CoreError::Journal("invalid rollout sequence".into()))?,
            thread_id: ThreadId::new(*thread_id),
            kind: (*kind).into(),
            payload: String::new(),
            occurred_at: Timestamp(
                recorded_at
                    .parse()
                    .map_err(|_| CoreError::Journal("invalid rollout timestamp".into()))?,
            ),
        }),
        _ => Err(CoreError::Journal("invalid rollout record".into())),
    }
}

fn recover_incomplete_tail(path: &Path) -> Result<(), CoreError> {
    let bytes = fs::read(path).map_err(io_error)?;
    let Some(last_newline) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        if !bytes.is_empty() {
            OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(io_error)?
                .set_len(0)
                .map_err(io_error)?;
        }
        return Ok(());
    };
    if last_newline + 1 != bytes.len() {
        OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(io_error)?
            .set_len((last_newline + 1) as u64)
            .map_err(io_error)?;
    }
    Ok(())
}

fn encode_hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn decode_hex(value: &str) -> Result<String, CoreError> {
    if !value.len().is_multiple_of(2) {
        return Err(CoreError::Journal("invalid rollout hex field".into()));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| CoreError::Journal("invalid rollout hex field".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| CoreError::Journal("invalid rollout utf-8 field".into()))
}
fn checksum(value: &str) -> String {
    let hash = value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("fnv1a64:{hash:016x}")
}

fn run_sql(database_path: &Path, sql: &str) -> Result<(), CoreError> {
    let result = Command::new("sqlite3")
        .arg(database_path)
        .arg(sql)
        .output()
        .map_err(io_error)?;
    if result.status.success() {
        Ok(())
    } else {
        Err(CoreError::Journal(
            String::from_utf8_lossy(&result.stderr).into_owned(),
        ))
    }
}
fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn io_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::Journal(error.to_string())
}
