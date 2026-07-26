use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

const EVENT_STREAM_FORMAT_VERSION: u32 = 1;

pub(crate) struct DecodedBatch<Event> {
    pub batch_id: String,
    pub stream_id: String,
    pub expected_sequence: u64,
    pub events: Vec<Event>,
}

#[derive(serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EncodedBatch {
    format_version: u32,
    stream_kind: String,
    batch_id: String,
    stream_id: String,
    expected_sequence: u64,
    events: Value,
    checksum: String,
}

pub(crate) fn read_batches<Event: DeserializeOwned>(
    path: &Path,
    stream_kind: &str,
) -> Result<Vec<DecodedBatch<Event>>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    discard_uncommitted_tail(path)?;
    BufReader::new(
        OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|error| error.to_string())?,
    )
    .lines()
    .filter_map(|line| match line {
        Ok(line) if line.trim().is_empty() => None,
        other => Some(other),
    })
    .map(|line| {
        let line = line.map_err(|error| error.to_string())?;
        let record: EncodedBatch = serde_json::from_str(&line)
            .map_err(|error| format!("invalid event stream: {error}"))?;
        if record.format_version != EVENT_STREAM_FORMAT_VERSION {
            return Err(format!(
                "unsupported event-stream format {}",
                record.format_version
            ));
        }
        if record.stream_kind != stream_kind {
            return Err(format!(
                "expected {stream_kind} stream, found {}",
                record.stream_kind
            ));
        }
        let checksum = batch_checksum(
            &record.stream_kind,
            &record.batch_id,
            &record.stream_id,
            record.expected_sequence,
            &record.events,
        )?;
        if checksum != record.checksum {
            return Err("event-stream checksum mismatch".into());
        }
        Ok(DecodedBatch {
            batch_id: record.batch_id,
            stream_id: record.stream_id,
            expected_sequence: record.expected_sequence,
            events: serde_json::from_value(record.events)
                .map_err(|error| format!("invalid typed event payload: {error}"))?,
        })
    })
    .collect()
}

pub(crate) fn append_batch<Event: Serialize>(
    path: &Path,
    stream_kind: &str,
    batch_id: &str,
    stream_id: &str,
    expected_sequence: u64,
    events: &[Event],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    discard_uncommitted_tail(path)?;
    let events = serde_json::to_value(events).map_err(|error| error.to_string())?;
    let record = EncodedBatch {
        format_version: EVENT_STREAM_FORMAT_VERSION,
        stream_kind: stream_kind.into(),
        batch_id: batch_id.into(),
        stream_id: stream_id.into(),
        expected_sequence,
        checksum: batch_checksum(stream_kind, batch_id, stream_id, expected_sequence, &events)?,
        events,
    };
    let encoded = serde_json::to_string(&record).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    writeln!(file, "{encoded}").map_err(|error| error.to_string())?;
    file.sync_data().map_err(|error| error.to_string())
}

fn batch_checksum(
    stream_kind: &str,
    batch_id: &str,
    stream_id: &str,
    expected_sequence: u64,
    events: &Value,
) -> Result<String, String> {
    let bytes = serde_json::to_vec(&(
        EVENT_STREAM_FORMAT_VERSION,
        stream_kind,
        batch_id,
        stream_id,
        expected_sequence,
        events,
    ))
    .map_err(|error| error.to_string())?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}

fn discard_uncommitted_tail(path: &Path) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.is_empty() || bytes.ends_with(b"\n") {
        return Ok(());
    }
    let committed_len = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    file.set_len(committed_len as u64)
        .map_err(|error| error.to_string())?;
    file.sync_data().map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;
