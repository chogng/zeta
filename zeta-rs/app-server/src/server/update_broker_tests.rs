use super::*;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionUpdate;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::TurnId;

#[test]
fn broker_fans_out_and_advances_each_connection_cursor() {
    let broker = UpdateBroker::default();
    let first = NotificationQueue::default();
    let second = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    broker.register(1, &first);
    broker.register(2, &second);
    broker.subscribe_session(1, session_id.clone(), 0);
    broker.subscribe_session(2, session_id.clone(), 1);
    let updates = vec![update(&session_id, 1), update(&session_id, 2)];

    broker.publish_session(&session_id, &updates);
    broker.publish_session(&session_id, &updates);

    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 1);
}

#[test]
fn broker_fans_out_filesystem_invalidation_without_a_subscription() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    broker.register(1, &queue);

    broker.publish_fs_changed(FsChanged::PathsChanged {
        paths: vec!["src/lib.rs".into()],
    });

    let notifications = queue.drain();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["method"], "fs/changed");
    assert_eq!(notifications[0]["params"]["paths"][0], "src/lib.rs");
}

#[test]
fn session_owned_thread_subscription_follows_session_lifecycle() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.register(1, &queue);
    broker.subscribe_session(1, session_id.clone(), 0);
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);

    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 1)]);
    assert_eq!(queue.len(), 1);
    let notifications = queue.drain();
    assert_eq!(notifications[0]["method"], "session/thread/update");

    broker.unsubscribe_session(1, &session_id);
    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 2)]);
    assert_eq!(queue.len(), 0);
}

#[test]
fn session_thread_subscription_can_be_removed_independently() {
    let broker = UpdateBroker::default();
    let queue = NotificationQueue::default();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    broker.register(1, &queue);
    broker.subscribe_session_thread(1, session_id.clone(), thread_id.clone(), 0);
    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 1)]);
    assert_eq!(queue.len(), 1);
    queue.drain();

    broker.unsubscribe_session_thread(1, &session_id, &thread_id);
    broker.publish_thread(&thread_id, &[thread_update(&session_id, &thread_id, 2)]);
    assert_eq!(queue.len(), 0);
}

fn update(session_id: &SessionId, sequence: u64) -> SessionUpdateEnvelope {
    SessionUpdateEnvelope {
        session_id: session_id.clone(),
        durable_sequence: sequence,
        update: SessionUpdate::Committed {
            event: SessionEvent::SessionCreated {
                session_id: session_id.clone(),
                title: "task".into(),
                model: None,
            },
        },
    }
}

fn thread_update(
    session_id: &SessionId,
    thread_id: &ThreadId,
    sequence: u64,
) -> ThreadUpdateEnvelope {
    ThreadUpdateEnvelope {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        durable_sequence: sequence,
        stream_cursor: None,
        update: ThreadUpdate::Committed {
            event: ThreadEvent::TurnCompleted {
                thread_id: thread_id.clone(),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        },
    }
}
