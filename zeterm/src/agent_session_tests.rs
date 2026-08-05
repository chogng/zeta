use super::{
    AgentSessionEvent, git_is_unavailable, snapshot_event_from_subscription, workspace_title,
};
use std::path::Path;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::session::{
    SessionSubscribeResult, SessionThreadProjection,
};
use zeta_protocol::{
    Session, SessionId, SessionStatus, SessionThread, SessionThreadStatus, Thread, ThreadEvent,
    ThreadId, ThreadOrigin, ThreadStatus, ThreadUpdate, ThreadUpdateEnvelope, TurnId,
};

#[test]
fn workspace_title_uses_the_last_path_component() {
    assert_eq!(workspace_title(Path::new("/work/zeta")), "zeta");
}

#[test]
fn workspace_title_has_a_stable_root_fallback() {
    assert_eq!(workspace_title(Path::new("/")), "Agent Session");
}

#[test]
fn git_unavailable_does_not_hide_operation_failures() {
    let server_error = |code| ClientError::Server {
        code,
        message: "Git error".into(),
    };

    assert!(git_is_unavailable(&server_error(-32060)));
    assert!(git_is_unavailable(&server_error(-32062)));
    assert!(!git_is_unavailable(&server_error(-32061)));
}

#[test]
fn subscription_snapshot_does_not_replay_history_as_live_thread_updates() {
    let session_id = SessionId::new("session-1").unwrap();
    let thread_id = ThreadId::new("thread-1").unwrap();
    let thread = Thread {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        title: "First terminal".to_owned(),
        status: ThreadStatus::Active,
        sequence: 1,
        turns: Vec::new(),
    };
    let historical_update = ThreadUpdateEnvelope {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        durable_sequence: 1,
        stream_cursor: None,
        update: ThreadUpdate::Committed {
            event: ThreadEvent::TurnCompleted {
                thread_id: thread_id.clone(),
                turn_id: TurnId::new("turn-1").unwrap(),
            },
        },
    };
    let subscription = SessionSubscribeResult {
        session: Session {
            session_id: session_id.clone(),
            title: "First terminal".to_owned(),
            status: SessionStatus::Active,
            model: None,
            sequence: 1,
            threads: vec![SessionThread {
                thread_id: thread_id.clone(),
                origin: ThreadOrigin::Root,
                status: SessionThreadStatus::Active,
            }],
        },
        updates: Vec::new(),
        thread_projections: vec![SessionThreadProjection {
            thread,
            updates: vec![historical_update],
        }],
    };

    let event = snapshot_event_from_subscription(&subscription, &thread_id, None).unwrap();
    assert!(matches!(event, AgentSessionEvent::Snapshot { .. }));
}
