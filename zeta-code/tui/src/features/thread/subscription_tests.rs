use super::ThreadSubscription;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;

#[test]
fn newer_update_for_active_scope_requests_snapshot() {
    let snapshot = thread("session-1", "thread-1", 4);
    let subscription = ThreadSubscription::from_snapshot(&snapshot);

    assert!(subscription.requires_snapshot(&update("session-1", "thread-1", 5)));
}

#[test]
fn duplicate_and_updates_for_another_thread_do_not_request_snapshot() {
    let snapshot = thread("session-1", "thread-1", 4);
    let subscription = ThreadSubscription::from_snapshot(&snapshot);

    assert!(!subscription.requires_snapshot(&update("session-1", "thread-1", 4)));
    assert!(!subscription.requires_snapshot(&update("session-1", "thread-old", 5)));
}

#[test]
fn session_mismatch_for_active_thread_requests_authoritative_snapshot() {
    let snapshot = thread("session-1", "thread-1", 4);
    let subscription = ThreadSubscription::from_snapshot(&snapshot);

    assert!(subscription.requires_snapshot(&update("session-old", "thread-1", 4)));
}

#[test]
fn confirming_a_new_snapshot_suppresses_buffered_duplicates() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);
    subscription.confirm_sequence(7);

    assert!(!subscription.requires_snapshot(&update("session-1", "thread-1", 6)));
    assert!(subscription.requires_snapshot(&update("session-1", "thread-1", 8)));
}

fn thread(session_id: &str, thread_id: &str, sequence: u64) -> Thread {
    Thread {
        session_id: SessionId::new(session_id).unwrap(),
        thread_id: ThreadId::new(thread_id).unwrap(),
        title: "Thread".into(),
        status: ThreadStatus::Active,
        sequence,
        turns: Vec::new(),
    }
}

fn update(session_id: &str, thread_id: &str, durable_sequence: u64) -> ThreadUpdateEnvelope {
    let session_id = SessionId::new(session_id).unwrap();
    let thread_id = ThreadId::new(thread_id).unwrap();
    ThreadUpdateEnvelope {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        durable_sequence,
        stream_cursor: None,
        update: ThreadUpdate::Committed {
            event: ThreadEvent::ThreadCreated {
                session_id,
                thread_id,
                title: "Thread".into(),
            },
        },
    }
}
