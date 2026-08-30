use super::*;
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::EventId;
use zeta_history::StoredEvent;
use zeta_history::Timestamp;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadStatus;

fn batch(expected_sequence: u64, event_sequence: u64) -> ThreadEventBatch {
    ThreadEventBatch {
        batch_id: "batch_1".into(),
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        expected_sequence,
        events: vec![StoredEvent {
            schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
            event_id: EventId("event_1".into()),
            sequence: event_sequence,
            thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
            recorded_at: Timestamp(1),
            command: None,
            event: ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1")
                    .expect("test ID is non-empty"),
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                title: "test".into(),
            },
        }],
        catalog: ThreadCatalogRecord {
            session_id: zeta_protocol::SessionId::new("session_1").expect("test ID is non-empty"),
            thread: SessionThread {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                title: "test".into(),
                created_at_unix_ms: 1,
                completed_turn_duration_ms: 0,
                active_turn_started_at_unix_ms: None,
                usage: Default::default(),
                parent_thread_id: None,
                forked_from_id: None,
                status: ThreadStatus::Active,
            },
            sequence: event_sequence,
            manager: SessionManagerInfo::default(),
            archived_at_unix_ms: None,
            stopped: false,
            requires_startup_recovery: false,
        },
    }
}

#[test]
fn validation_rejects_stale_expected_sequence() {
    assert_eq!(
        validate_append_batch(&batch(3, 4), 2),
        Err(ThreadStoreError::SequenceConflict {
            expected: 3,
            actual: 2
        })
    );
}

#[test]
fn validation_rejects_event_sequence_outside_the_batch() {
    assert!(matches!(
        validate_append_batch(&batch(0, 2), 0),
        Err(ThreadStoreError::InvalidBatch(_))
    ));
}
