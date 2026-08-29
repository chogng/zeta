use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::EventId;
use zeta_history::StoredEvent;
use zeta_history::Timestamp;
use zeta_protocol::{
    SessionEvent, SessionId, SessionThread, SessionThreadStatus, ThreadEvent, ThreadId,
    ThreadOrigin,
};
use zeta_rollout::LocalStateRepository;
use zeta_session_store::{
    CURRENT_SESSION_EVENT_SCHEMA_VERSION, SessionEventBatch, SessionEventId, SessionTimestamp,
    StoredSessionEvent,
};
use zeta_state::StateRuntime;
use zeta_thread_store::ThreadEventBatch;

fn temporary_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-rollout-trace-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn trace_keeps_session_and_thread_sequences_independent() {
    let root = temporary_root();
    let state = StateRuntime::open(&root).unwrap();
    let repository = LocalStateRepository::open(&state).unwrap();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    let session_store = repository.session_store();
    let thread_store = repository.thread_store();

    session_store
        .append_batch(&SessionEventBatch {
            batch_id: "session_batch".into(),
            session_id: session_id.clone(),
            expected_sequence: 0,
            events: vec![
                StoredSessionEvent {
                    schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
                    event_id: SessionEventId("session_event_1".into()),
                    sequence: 1,
                    session_id: session_id.clone(),
                    recorded_at: SessionTimestamp(1),
                    command: None,
                    event: SessionEvent::SessionCreated {
                        session_id: session_id.clone(),
                        title: "Task".into(),
                        model: None,
                        workspace: None,
                    },
                },
                StoredSessionEvent {
                    schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
                    event_id: SessionEventId("session_event_2".into()),
                    sequence: 2,
                    session_id: session_id.clone(),
                    recorded_at: SessionTimestamp(2),
                    command: None,
                    event: SessionEvent::ThreadCreationPlanned {
                        session_id: session_id.clone(),
                        thread: SessionThread {
                            thread_id: thread_id.clone(),
                            origin: ThreadOrigin::Root,
                            status: SessionThreadStatus::Creating,
                        },
                        title: "Primary branch".into(),
                    },
                },
            ],
        })
        .unwrap();
    thread_store
        .append_batch(&ThreadEventBatch {
            batch_id: "thread_batch".into(),
            thread_id: thread_id.clone(),
            expected_sequence: 0,
            events: vec![StoredEvent {
                schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
                event_id: EventId("thread_event_1".into()),
                sequence: 1,
                thread_id: thread_id.clone(),
                recorded_at: Timestamp(3),
                command: None,
                event: ThreadEvent::ThreadCreated {
                    session_id: session_id.clone(),
                    thread_id: thread_id.clone(),
                    title: "Primary branch".into(),
                },
            }],
        })
        .unwrap();

    let trace =
        capture_session_trace(session_store.as_ref(), thread_store.as_ref(), &session_id).unwrap();

    assert_eq!(trace.format_version, ROLLOUT_TRACE_FORMAT_VERSION);
    assert_eq!(trace.session_events.last().unwrap().sequence, 2);
    assert_eq!(trace.threads[0].events[0].sequence, 1);
    assert_eq!(trace.threads[0].thread_id, thread_id);
    fs::remove_dir_all(root).unwrap();
}
