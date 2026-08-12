use super::ThreadSubscription;
use super::ThreadUpdateDisposition;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_protocol::SessionId;
use zeta_protocol::StreamCursor;
use zeta_protocol::StreamInstanceId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;

#[derive(Clone)]
struct RecordingTransport {
    responses: VecDeque<String>,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl JsonRpcTransport for RecordingTransport {
    fn round_trip(&mut self, request: &str) -> Result<String, ClientError> {
        self.requests
            .lock()
            .expect("request log is not poisoned")
            .push(serde_json::from_str(request).expect("request is valid JSON"));
        self.responses
            .pop_front()
            .ok_or_else(|| ClientError::Transport("no response".into()))
    }
}

#[test]
fn newer_update_for_active_scope_requests_snapshot() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);

    assert_eq!(
        subscription.classify_update(&update("session-1", "thread-1", 5)),
        ThreadUpdateDisposition::RefreshSnapshot
    );
}

#[test]
fn duplicate_and_updates_for_another_thread_do_not_request_snapshot() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);

    assert_eq!(
        subscription.classify_update(&update("session-1", "thread-1", 4)),
        ThreadUpdateDisposition::Ignore
    );
    assert_eq!(
        subscription.classify_update(&update("session-1", "thread-old", 5)),
        ThreadUpdateDisposition::Ignore
    );
}

#[test]
fn session_mismatch_for_active_thread_requests_authoritative_snapshot() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);

    assert_eq!(
        subscription.classify_update(&update("session-old", "thread-1", 4)),
        ThreadUpdateDisposition::RefreshSnapshot
    );
}

#[test]
fn confirming_a_new_snapshot_suppresses_buffered_duplicates() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);
    subscription.confirm_sequence(7);

    assert_eq!(
        subscription.classify_update(&update("session-1", "thread-1", 6)),
        ThreadUpdateDisposition::Ignore
    );
    assert_eq!(
        subscription.classify_update(&update("session-1", "thread-1", 8)),
        ThreadUpdateDisposition::RefreshSnapshot
    );
}

#[test]
fn transient_updates_are_applied_once_in_stream_order() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);
    let transient = transient_update("stream-1", 1);

    assert_eq!(
        subscription.classify_update(&transient),
        ThreadUpdateDisposition::ApplyTransientAfterReset
    );
    assert_eq!(
        subscription.classify_update(&transient),
        ThreadUpdateDisposition::Ignore
    );
    assert_eq!(
        subscription.classify_update(&transient_update("stream-1", 2)),
        ThreadUpdateDisposition::ApplyTransient
    );
}

#[test]
fn transient_gap_resets_projection_and_requests_snapshot() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);
    assert_eq!(
        subscription.classify_update(&transient_update("stream-1", 1)),
        ThreadUpdateDisposition::ApplyTransientAfterReset
    );

    assert_eq!(
        subscription.classify_update(&transient_update("stream-1", 3)),
        ThreadUpdateDisposition::ResetTransientAndRefreshSnapshot
    );
    subscription.confirm_sequence(5);
    let mut next = transient_update("stream-1", 4);
    next.durable_sequence = 5;
    assert_eq!(
        subscription.classify_update(&next),
        ThreadUpdateDisposition::ApplyTransient
    );
}

#[test]
fn new_stream_instance_resets_the_previous_transient_projection() {
    let snapshot = thread("session-1", "thread-1", 4);
    let mut subscription = ThreadSubscription::from_snapshot(&snapshot);
    assert_eq!(
        subscription.classify_update(&transient_update("stream-1", 1)),
        ThreadUpdateDisposition::ApplyTransientAfterReset
    );

    assert_eq!(
        subscription.classify_update(&transient_update("stream-2", 1)),
        ThreadUpdateDisposition::ApplyTransientAfterReset
    );
}

#[test]
fn switching_threads_unsubscribes_the_previous_session_and_thread() {
    let previous = thread("session-1", "thread-1", 4);
    let next = thread("session-2", "thread-2", 7);
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mut client = AppServerClient::new(RecordingTransport {
        responses: VecDeque::from([
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": { "thread": next, "updates": [] }
            })
            .to_string(),
            serde_json::json!({ "jsonrpc": "2.0", "id": 2, "result": null }).to_string(),
        ]),
        requests: Arc::clone(&requests),
    });
    let mut subscription = ThreadSubscription::from_snapshot(&previous);

    subscription
        .switch(
            &mut client,
            &SessionId::new("session-2").unwrap(),
            &ThreadId::new("thread-2").unwrap(),
        )
        .expect("thread switch should succeed");

    let requests = requests.lock().expect("request log is not poisoned");
    assert_eq!(requests[1]["method"], "session/thread/unsubscribe");
    assert_eq!(requests[1]["params"]["sessionId"], "session-1");
    assert_eq!(requests[1]["params"]["threadId"], "thread-1");
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

fn transient_update(stream_id: &str, sequence: u64) -> ThreadUpdateEnvelope {
    ThreadUpdateEnvelope {
        session_id: SessionId::new("session-1").unwrap(),
        thread_id: ThreadId::new("thread-1").unwrap(),
        durable_sequence: 4,
        stream_cursor: Some(StreamCursor {
            stream_instance_id: StreamInstanceId::new(stream_id).unwrap(),
            sequence,
        }),
        update: ThreadUpdate::ItemDelta {
            turn_id: zeta_protocol::TurnId::new("turn-1").unwrap(),
            item_id: zeta_protocol::ItemId::new("item-1").unwrap(),
            delta: zeta_protocol::ItemDelta::AgentMessage { text: "x".into() },
        },
    }
}
