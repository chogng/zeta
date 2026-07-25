use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_core::EventJournal;
use zeta_core::{IdempotencyLedger, IdempotencyRecord};
use zeta_protocol::{AgentEvent, EventId, ThreadId, Timestamp};

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn rollout_rejects_duplicate_event_and_rebuilds_projection() {
    let directory = temp_path("rollout");
    let log = RolloutLog::open(directory.join("history.rollout")).unwrap();
    let event = AgentEvent {
        event_id: EventId("event_1".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread_1"),
        kind: "thread.started".into(),
        payload: "test".into(),
        occurred_at: Timestamp(1),
    };
    log.append(&event).unwrap();
    let other_thread_event = AgentEvent {
        event_id: EventId("event_2".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread_2"),
        kind: "thread.started".into(),
        payload: "test".into(),
        occurred_at: Timestamp(2),
    };
    log.append(&other_thread_event).unwrap();
    assert!(log.append(&event).is_err());
    log.rebuild_sqlite_projection(directory.join("state.sqlite"))
        .unwrap();
    assert!(directory.join("state.sqlite").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn thread_store_uses_independent_rollouts() {
    let directory = temp_path("thread-store");
    let store = ThreadRolloutStore::open(&directory).unwrap();
    let first = AgentEvent {
        event_id: EventId("event_1".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread/one"),
        kind: "thread.started".into(),
        payload: "first".into(),
        occurred_at: Timestamp(1),
    };
    let second = AgentEvent {
        event_id: EventId("event_2".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread/two"),
        kind: "thread.started".into(),
        payload: "second".into(),
        occurred_at: Timestamp(1),
    };
    store.append(&first).unwrap();
    store.append(&second).unwrap();
    assert_eq!(store.read_thread(&first.thread_id).unwrap(), vec![first]);
    assert_eq!(store.read_thread(&second.thread_id).unwrap(), vec![second]);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn file_idempotency_ledger_survives_reopen() {
    let directory = temp_path("idempotency");
    let path = directory.join("ledger");
    let ledger = FileIdempotencyLedger::open(&path).unwrap();
    let record = IdempotencyRecord {
        method: "thread/start".into(),
        key: "key".into(),
        parameters: "{}".into(),
        result: "{\"threadId\":\"one\"}".into(),
    };
    ledger.put(record.clone()).unwrap();
    assert_eq!(
        FileIdempotencyLedger::open(path)
            .unwrap()
            .get("thread/start", "key")
            .unwrap(),
        Some(record)
    );
    fs::remove_dir_all(directory).unwrap();
}
