use super::*;
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::EventId;
use zeta_history::StoredEvent;
use zeta_history::Timestamp;
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

#[test]
fn history_page_walks_backward_without_reordering_events() {
    let thread_id = ThreadId::new("thread_1").unwrap();
    let session_id = zeta_protocol::SessionId::new("session_1").unwrap();
    let events = (1..=5)
        .map(|sequence| StoredEvent {
            schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
            event_id: EventId(format!("event_{sequence}")),
            sequence,
            thread_id: thread_id.clone(),
            recorded_at: Timestamp(sequence.into()),
            command: None,
            event: ThreadEvent::ThreadCreated {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                title: format!("thread {sequence}"),
            },
        })
        .collect::<Vec<_>>();

    let newest = history_page_from_events(
        &events,
        ThreadHistoryQuery {
            before_sequence: None,
            limit: 2,
        },
    )
    .unwrap();
    assert_eq!(
        newest
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![4, 5]
    );
    assert_eq!(newest.next_before_sequence, Some(4));

    let older = history_page_from_events(
        &events,
        ThreadHistoryQuery {
            before_sequence: newest.next_before_sequence,
            limit: 2,
        },
    )
    .unwrap();
    assert_eq!(
        older
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(older.next_before_sequence, Some(2));
}

#[test]
fn history_query_rejects_zero_limit_and_zero_cursor() {
    let events = Vec::new();
    assert!(matches!(
        history_page_from_events(
            &events,
            ThreadHistoryQuery {
                before_sequence: None,
                limit: 0,
            }
        ),
        Err(ThreadStoreError::InvalidQuery(_))
    ));
    assert!(matches!(
        history_page_from_events(
            &events,
            ThreadHistoryQuery {
                before_sequence: Some(0),
                limit: 1,
            }
        ),
        Err(ThreadStoreError::InvalidQuery(_))
    ));
}
