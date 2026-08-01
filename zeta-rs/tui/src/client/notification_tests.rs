use super::ClientEvent;
use super::map_event;
use zeta_app_server_client::{AppServerEvent, ConnectionCloseReason, ServerNotification};
use zeta_app_server_protocol::protocol::git::{GitHeadDto, GitStatusChanged, GitStatusResult};
use zeta_app_server_protocol::protocol::notification::{SkillsChanged, ThreadUpdateEnvelope};
use zeta_protocol::{SessionId, StreamInstanceId, ThreadEvent, ThreadId, ThreadUpdate};

#[test]
fn skills_changed_is_mapped_without_exposing_the_wire_notification() {
    assert_eq!(
        map_event(AppServerEvent::Notification(
            ServerNotification::SkillsChanged(SkillsChanged { generation: 7 })
        )),
        Some(ClientEvent::SkillsChanged)
    );
}

#[test]
fn thread_update_preserves_typed_scope_and_sequence() {
    let session_id = SessionId::new("session-1").unwrap();
    let thread_id = ThreadId::new("thread-1").unwrap();
    let update = ThreadUpdateEnvelope {
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        durable_sequence: 1,
        stream_cursor: None,
        update: ThreadUpdate::Committed {
            event: ThreadEvent::ThreadCreated {
                session_id,
                thread_id,
                title: "Thread".into(),
            },
        },
    };
    let Some(ClientEvent::ThreadUpdated(update)) = map_event(AppServerEvent::Notification(
        ServerNotification::ThreadUpdate(Box::new(update)),
    )) else {
        panic!("typed Thread update should be preserved");
    };
    assert_eq!(update.session_id.as_str(), "session-1");
    assert_eq!(update.thread_id.as_str(), "thread-1");
    assert_eq!(update.durable_sequence, 1);
}

#[test]
fn git_status_change_is_explicitly_ignored_until_the_tui_owns_git_state() {
    let changed = GitStatusChanged {
        status: GitStatusResult {
            stream_instance_id: StreamInstanceId::new("git-stream").unwrap(),
            revision: 1,
            workspace_path: String::new(),
            head: GitHeadDto::Unborn {
                name: "main".into(),
            },
            changes: Vec::new(),
        },
    };

    assert_eq!(
        map_event(AppServerEvent::Notification(
            ServerNotification::GitStatusChanged(changed)
        )),
        None
    );
}

#[test]
fn notification_failure_becomes_a_client_event() {
    assert_eq!(
        map_event(AppServerEvent::ConnectionClosed(
            ConnectionCloseReason::DriverStopped
        )),
        Some(ClientEvent::Failed(
            "App Server connection closed: DriverStopped".into()
        ))
    );
}
