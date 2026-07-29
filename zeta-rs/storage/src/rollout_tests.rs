use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_protocol::{StableTurnError, ThreadEvent, ThreadId, TurnId};
use zeta_thread_store::{
    CURRENT_STORED_EVENT_SCHEMA_VERSION, EventId, StoredEvent, ThreadEventBatch, ThreadStore,
    ThreadStoreError, Timestamp,
};

fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn batch(batch_id: &str, event: StoredEvent) -> ThreadEventBatch {
    ThreadEventBatch {
        batch_id: batch_id.into(),
        thread_id: event.thread_id.clone(),
        expected_sequence: event.sequence - 1,
        events: vec![event],
    }
}

#[test]
fn thread_store_rejects_duplicate_event_and_rebuilds_projection() {
    let directory = temp_path("rollout");
    let store = ThreadRolloutStore::open(&directory).unwrap();
    let event = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event_1".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        recorded_at: Timestamp(1),
        command: None,
        event: ThreadEvent::ThreadCreated {
            session_id: zeta_protocol::SessionId::new("session_1").expect("test ID is non-empty"),
            thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
            title: "test".into(),
        },
    };
    store
        .append_batch(&batch("batch_1", event.clone()))
        .unwrap();
    let other_thread_event = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event_2".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread_2").expect("test ID is non-empty"),
        recorded_at: Timestamp(2),
        command: None,
        event: ThreadEvent::ThreadCreated {
            session_id: zeta_protocol::SessionId::new("session_1").expect("test ID is non-empty"),
            thread_id: ThreadId::new("thread_2").expect("test ID is non-empty"),
            title: "test".into(),
        },
    };
    store
        .append_batch(&batch("batch_2", other_thread_event))
        .unwrap();
    let duplicate_batch_id = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event_3".into()),
        sequence: 2,
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        recorded_at: Timestamp(3),
        command: None,
        event: ThreadEvent::TurnAccepted {
            thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
            turn_id: zeta_protocol::TurnId::new("turn_1").expect("test ID is non-empty"),
            model: None,
        },
    };
    assert!(matches!(
        store.append_batch(&batch("batch_1", duplicate_batch_id)),
        Err(ThreadStoreError::InvalidBatch(_))
    ));
    assert!(matches!(
        store.append_batch(&batch("batch_3", event.clone())),
        Err(ThreadStoreError::SequenceConflict {
            expected: 0,
            actual: 1
        })
    ));
    store
        .rebuild_sqlite_projection(directory.join("state.sqlite"))
        .unwrap();
    assert!(directory.join("state.sqlite").exists());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn typed_failure_payload_survives_round_trip() {
    let directory = temp_path("typed-failure");
    let store = ThreadRolloutStore::open(&directory).unwrap();
    let event = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event_1".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        recorded_at: Timestamp(1),
        command: None,
        event: ThreadEvent::TurnFailed {
            thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
            turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            error: StableTurnError::model_invocation_failed(),
        },
    };

    store
        .append_batch(&batch("batch_1", event.clone()))
        .unwrap();

    assert_eq!(
        store
            .load(&ThreadId::new("thread_1").expect("test ID is non-empty"))
            .unwrap(),
        vec![event]
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn thread_store_uses_independent_rollouts() {
    let directory = temp_path("thread-store");
    let store = ThreadRolloutStore::open(&directory).unwrap();
    let first = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event_1".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread/one").expect("test ID is non-empty"),
        recorded_at: Timestamp(1),
        command: None,
        event: ThreadEvent::ThreadCreated {
            session_id: zeta_protocol::SessionId::new("session_1").expect("test ID is non-empty"),
            thread_id: ThreadId::new("thread/one").expect("test ID is non-empty"),
            title: "first".into(),
        },
    };
    let second = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event_2".into()),
        sequence: 1,
        thread_id: ThreadId::new("thread/two").expect("test ID is non-empty"),
        recorded_at: Timestamp(1),
        command: None,
        event: ThreadEvent::ThreadCreated {
            session_id: zeta_protocol::SessionId::new("session_1").expect("test ID is non-empty"),
            thread_id: ThreadId::new("thread/two").expect("test ID is non-empty"),
            title: "second".into(),
        },
    };
    store
        .append_batch(&batch("batch_1", first.clone()))
        .unwrap();
    store
        .append_batch(&batch("batch_2", second.clone()))
        .unwrap();
    assert_eq!(store.read_thread(&first.thread_id).unwrap(), vec![first]);
    assert_eq!(store.read_thread(&second.thread_id).unwrap(), vec![second]);
    fs::remove_dir_all(directory).unwrap();
}
