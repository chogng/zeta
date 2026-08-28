use tempfile::tempdir;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_workspace::WorkspaceBinding;
use zeta_workspace::WorkspaceRoot;

use super::SessionWorkspaceRoute;
use super::route_session_workspace;

fn session(workspace: Option<WorkspaceBinding>) -> Session {
    Session {
        session_id: SessionId::new("session-route").unwrap(),
        title: "Route".into(),
        status: SessionStatus::Active,
        model: None,
        workspace,
        next_approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
        current_thread_id: None,
        sequence: 1,
        threads: Vec::new(),
    }
}

#[test]
fn routes_current_foreign_and_legacy_sessions() {
    let root = tempdir().unwrap();
    let current_path = root.path().join("current");
    let foreign_path = root.path().join("foreign");
    std::fs::create_dir(&current_path).unwrap();
    std::fs::create_dir(&foreign_path).unwrap();
    let current = WorkspaceRoot::open(&current_path).unwrap();
    let foreign = WorkspaceRoot::open(&foreign_path).unwrap();

    assert_eq!(
        route_session_workspace(
            &session(Some(WorkspaceBinding::from_root(&current))),
            &current_path,
        )
        .unwrap(),
        SessionWorkspaceRoute::Current,
    );
    assert_eq!(
        route_session_workspace(
            &session(Some(WorkspaceBinding::from_root(&foreign))),
            &current_path,
        )
        .unwrap(),
        SessionWorkspaceRoute::Reconnect(WorkspaceBinding::from_root(&foreign)),
    );
    assert_eq!(
        route_session_workspace(&session(None), &current_path).unwrap(),
        SessionWorkspaceRoute::LegacyUnbound,
    );
}
