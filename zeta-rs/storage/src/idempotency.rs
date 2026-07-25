use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use zeta_core::{CoreError, IdempotencyLedger, IdempotencyRecord};

/// File-backed idempotency ledger for one local state root.
pub struct FileIdempotencyLedger {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl FileIdempotencyLedger {
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
        Ok(Self {
            path,
            write_lock: Mutex::new(()),
        })
    }

    fn records(&self) -> Result<Vec<IdempotencyRecord>, CoreError> {
        BufReader::new(
            OpenOptions::new()
                .read(true)
                .open(&self.path)
                .map_err(io_error)?,
        )
        .lines()
        .map(|line| parse(&line.map_err(io_error)?))
        .collect()
    }
}

impl IdempotencyLedger for FileIdempotencyLedger {
    fn get(&self, method: &str, key: &str) -> Result<Option<IdempotencyRecord>, CoreError> {
        Ok(self
            .records()?
            .into_iter()
            .find(|record| record.method == method && record.key == key))
    }
    fn put(&self, record: IdempotencyRecord) -> Result<(), CoreError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| CoreError::Journal("idempotency write lock poisoned".into()))?;
        if self.get(&record.method, &record.key)?.is_some() {
            return Err(CoreError::Journal("idempotency key already exists".into()));
        }
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(io_error)?;
        writeln!(
            file,
            "{}\t{}\t{}\t{}",
            hex(&record.method),
            hex(&record.key),
            hex(&record.parameters),
            hex(&record.result)
        )
        .map_err(io_error)?;
        file.sync_data().map_err(io_error)
    }
}

fn parse(line: &str) -> Result<IdempotencyRecord, CoreError> {
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() != 4 {
        return Err(CoreError::Journal("invalid idempotency record".into()));
    }
    Ok(IdempotencyRecord {
        method: unhex(fields[0])?,
        key: unhex(fields[1])?,
        parameters: unhex(fields[2])?,
        result: unhex(fields[3])?,
    })
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn unhex(value: &str) -> Result<String, CoreError> {
    if !value.len().is_multiple_of(2) {
        return Err(CoreError::Journal("invalid idempotency hex".into()));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| CoreError::Journal("invalid idempotency hex".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| CoreError::Journal("invalid idempotency utf-8".into()))
}
fn io_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::Journal(error.to_string())
}
