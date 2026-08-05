use super::SessionSelectionAction;
use super::session_selection_view;
use crate::components::selection::SelectionInputOutcome;
use crate::components::selection::SelectionViewState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
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

#[test]
fn resume_pane_groups_statuses_and_activates_the_selected_session() {
    let sessions = vec![
        Session {
            session_id: SessionId::new("session-1").unwrap(),
            title: "Active work".into(),
            status: SessionStatus::Active,
            model: None,
            sequence: 1,
            threads: Vec::new(),
        },
        Session {
            session_id: SessionId::new("session-2").unwrap(),
            title: "Completed work".into(),
            status: SessionStatus::Completed,
            model: None,
            sequence: 2,
            threads: Vec::new(),
        },
        Session {
            session_id: SessionId::new("session-3").unwrap(),
            title: "Archived work".into(),
            status: SessionStatus::Archived,
            model: None,
            sequence: 3,
            threads: Vec::new(),
        },
    ];

    let view = session_selection_view(&sessions, "session-2");
    let mut state = SelectionViewState::new(view.model.into_body());

    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        ["All (3)", "Active (1)", "Completed (1)", "Archived (1)"]
    );
    assert_eq!(state.selected_item().unwrap().label(), "Completed work ✓");

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        SelectionInputOutcome::Consumed
    );
    assert_eq!(state.tabs()[state.active_tab_index()].label(), "Active (1)");
    let SelectionInputOutcome::Activate(item_id) =
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("active session selection should produce an activation");
    };
    assert_eq!(
        view.actions.get(&item_id),
        Some(&SessionSelectionAction::Resume {
            session_id: "session-1".into(),
        })
    );
}
