//! Product Agent session contract tests.

use super::AgentSession;
use super::AgentSessionCommand;
use super::AgentSessionConnectionLost;
use super::AgentSessionEvent;
use super::client_error;
use super::git_is_unavailable;
use super::route_session_for_target;
use super::shell_completion_sources_changed;
use super::snapshot_event_from_subscription;
use super::workspace_title;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_app_server_protocol::protocol::session::SessionSubscribeResult;
use zeta_app_server_protocol::protocol::session::SessionThreadProjection;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::Thread;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadOrigin;

use crate::app_server::{ClientError, SessionWorkspaceRoute};
use zeta_protocol::ThreadStatus;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::TurnId;
use zeta_protocol::WorkspaceBinding;
use zeta_workspace::WorkspaceRoot;

#[test]
fn session_route_distinguishes_current_and_foreign_workspaces() {
    let root = tempfile::tempdir().unwrap();
    let current_path = root.path().join("current");
    let foreign_path = root.path().join("foreign");
    std::fs::create_dir(&current_path).unwrap();
    std::fs::create_dir(&foreign_path).unwrap();
    let current = WorkspaceRoot::open(&current_path).unwrap();
    let foreign = WorkspaceRoot::open(&foreign_path).unwrap();
    let mut session = session_with_workspace(WorkspaceBinding::from_root(&current));

    assert!(matches!(
        route_session_for_target(&session, &current_path).unwrap(),
        SessionWorkspaceRoute::Current
    ));
    session.workspace = Some(WorkspaceBinding::from_root(&foreign));
    assert!(matches!(
        route_session_for_target(&session, &current_path).unwrap(),
        SessionWorkspaceRoute::Reconnect(binding) if binding.root() == foreign.canonical_path()
    ));
}

fn session_with_workspace(workspace: WorkspaceBinding) -> Session {
    Session {
        session_id: SessionId::new("routed-session").unwrap(),
        title: "Routed".into(),
        status: SessionStatus::Active,
        model: None,
        workspace: Some(workspace),
        sequence: 1,
        threads: Vec::new(),
    }
}

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
fn client_transport_errors_preserve_the_reconnect_marker() {
    let transport = client_error(ClientError::Transport("SSH closed".into()));
    assert!(
        transport
            .downcast_ref::<AgentSessionConnectionLost>()
            .is_some()
    );

    let rejected = client_error(ClientError::Server {
        code: -32000,
        message: "request rejected".into(),
    });
    assert!(
        rejected
            .downcast_ref::<AgentSessionConnectionLost>()
            .is_none()
    );
}

#[test]
fn disconnected_agent_rejects_commands_before_they_enter_the_queue() {
    let (commands, receiver) = mpsc::sync_channel(2);
    let available = Arc::new(AtomicBool::new(false));
    let session = AgentSession {
        available: Arc::clone(&available),
        commands,
        worker: None,
    };

    assert!(
        session
            .refresh()
            .unwrap_err()
            .to_string()
            .contains("not connected")
    );
    assert!(receiver.try_recv().is_err());

    available.store(true, Ordering::Release);
    session.refresh().unwrap();
    assert!(matches!(
        receiver.recv().unwrap(),
        AgentSessionCommand::Refresh
    ));
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
        usage: Default::default(),
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
            workspace: None,
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
        agent_tree: Default::default(),
    };

    let event = snapshot_event_from_subscription(&subscription, &thread_id, None).unwrap();
    assert!(matches!(event, AgentSessionEvent::Snapshot { .. }));
}

#[test]
fn shell_completion_sources_refresh_only_for_relevant_workspace_changes() {
    assert!(shell_completion_sources_changed(
        &FsChanged::RescanRequired {
            workspace_folder_id: None,
        }
    ));
    assert!(shell_completion_sources_changed(&FsChanged::PathsChanged {
        workspace_folder_id: None,
        paths: vec![PathBuf::from("frontend/package.json")],
    }));
    assert!(shell_completion_sources_changed(&FsChanged::PathsChanged {
        workspace_folder_id: None,
        paths: vec![PathBuf::from("Justfile")],
    }));
    assert!(!shell_completion_sources_changed(
        &FsChanged::PathsChanged {
            workspace_folder_id: None,
            paths: vec![PathBuf::from("src/main.rs")],
        }
    ));
}
