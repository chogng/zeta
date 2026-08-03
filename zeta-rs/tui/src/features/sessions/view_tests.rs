use super::SessionSelectionAction;
use super::session_selection_view;
use crate::components::selection::SelectionViewState;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;

#[test]
fn resume_pane_marks_the_current_session_and_maps_enter_to_its_id() {
    let sessions = vec![Session {
        session_id: SessionId::new("session-1").unwrap(),
        title: "Current work".into(),
        status: SessionStatus::Active,
        model: None,
        sequence: 1,
        threads: Vec::new(),
    }];

    let view = session_selection_view(&sessions, "session-1");
    let state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.title(), "Resume session");
    assert_eq!(state.tabs()[0].label(), "All (1)");
    assert_eq!(state.visible_items()[0].label(), "Current work ✓");
    assert_eq!(
        view.actions.values().next(),
        Some(&SessionSelectionAction::Resume {
            session_id: "session-1".into(),
        })
    );
}
