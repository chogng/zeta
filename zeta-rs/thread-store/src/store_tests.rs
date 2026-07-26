use super::*;
use crate::EventId;
use crate::Timestamp;
use zeta_protocol::ThreadEvent;

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
