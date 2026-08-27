use super::PaneInput;
use super::TabInputChange;
use super::TabInputKey;
use super::WorkbenchController;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;

fn session(id: &str, title: &str) -> Session {
    Session {
        session_id: SessionId::new(id).expect("test Session ID must be valid"),
        title: title.to_owned(),
        status: SessionStatus::Active,
        model: None,
        workspace: None,
        sequence: 1,
        threads: Vec::new(),
    }
}

#[test]
fn session_conversion_and_default_pane_policy_belong_to_the_controller() {
    let mut controller = WorkbenchController::new();
    let session = session("session-1", "First session");
    let tab_key = TabInputKey::session(session.session_id.clone());

    assert_eq!(
        controller
            .workbench_mut()
            .upsert_session(&session, "workspace"),
        TabInputChange::Added(tab_key.clone())
    );

    let tab = controller
        .workbench()
        .tab_part()
        .input(&tab_key)
        .expect("Session tab must exist");
    assert_eq!(tab.title(), "First session");
    assert_eq!(tab.workspace(), "workspace");
    assert_eq!(tab.status_label(), "Active");
    assert_eq!(
        controller
            .workbench()
            .active_pane()
            .and_then(|pane| pane.input().terminal_session_id().cloned()),
        Some(session.session_id)
    );
}

#[test]
fn workspace_return_state_is_controller_owned_and_removed_with_its_tab() {
    let mut controller = WorkbenchController::new();
    let session = session("session-1", "First session");
    let tab_key = TabInputKey::session(session.session_id.clone());
    controller
        .workbench_mut()
        .upsert_session(&session, "workspace");

    assert!(
        controller
            .workbench_mut()
            .remember_workspace_return(&tab_key, PaneInput::settings())
    );
    assert!(controller.close_tab(&tab_key).is_some());
    assert!(
        controller
            .workbench_mut()
            .take_workspace_return(&tab_key)
            .is_none()
    );
}
