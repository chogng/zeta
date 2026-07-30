use super::*;
use zeta_protocol::{SessionEvent, SessionUpdate};

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
