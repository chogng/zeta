use super::*;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct TestEvent {
    sequence: u64,
}

static NEXT_TEMPORARY_PATH: AtomicU64 = AtomicU64::new(1);

fn path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-event-stream-{}-{}-{}.rollout",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_TEMPORARY_PATH.fetch_add(1, Ordering::Relaxed),
    ))
}

#[test]
fn one_engine_round_trips_different_typed_stream_kinds() {
    let session_path = path();
    let thread_path = path();
    append_batch(
        &session_path,
        "session",
        "batch_1",
        "session_1",
        0,
        &[TestEvent { sequence: 1 }],
    )
    .unwrap();
    append_batch(
        &thread_path,
        "thread",
        "batch_2",
        "thread_1",
        0,
        &[TestEvent { sequence: 1 }],
    )
    .unwrap();

    assert_eq!(
        read_batches::<TestEvent>(&session_path, "session").unwrap()[0].events,
        vec![TestEvent { sequence: 1 }]
    );
    assert!(read_batches::<TestEvent>(&thread_path, "session").is_err());
    fs::remove_file(session_path).unwrap();
    fs::remove_file(thread_path).unwrap();
}

#[test]
fn a_batch_is_one_committed_physical_record() {
    let stream_path = path();
    append_batch(
        &stream_path,
        "thread",
        "batch_1",
        "thread_1",
        0,
        &[TestEvent { sequence: 1 }, TestEvent { sequence: 2 }],
    )
    .unwrap();

    assert_eq!(fs::read_to_string(&stream_path).unwrap().lines().count(), 1);
    assert_eq!(
        read_batches::<TestEvent>(&stream_path, "thread").unwrap()[0]
            .events
            .len(),
        2
    );
    fs::remove_file(stream_path).unwrap();
}

#[test]
fn reads_discard_only_an_unterminated_tail() {
    let stream_path = path();
    append_batch(
        &stream_path,
        "thread",
        "batch_1",
        "thread_1",
        0,
        &[TestEvent { sequence: 1 }],
    )
    .unwrap();
    let mut file = OpenOptions::new().append(true).open(&stream_path).unwrap();
    write!(file, "{{\"incomplete\":").unwrap();
    file.sync_data().unwrap();

    assert_eq!(
        read_batches::<TestEvent>(&stream_path, "thread").unwrap()[0].events,
        vec![TestEvent { sequence: 1 }]
    );
    assert_eq!(fs::read_to_string(&stream_path).unwrap().lines().count(), 1);
    fs::remove_file(stream_path).unwrap();
}

#[test]
fn committed_corruption_is_rejected() {
    let stream_path = path();
    fs::write(&stream_path, "{\"formatVersion\":0}\n").unwrap();

    assert!(read_batches::<TestEvent>(&stream_path, "thread").is_err());
    fs::remove_file(stream_path).unwrap();
}
