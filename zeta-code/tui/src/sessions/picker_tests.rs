use super::SessionSelectionAction;
use super::session_choices;
use super::session_choices_at;
use crate::widgets::list_selection::ListSelectionInputOutcome;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[test]
fn resume_picker_marks_the_current_session_and_maps_enter_to_its_id() {
    let sessions = vec![Session {
        session_id: SessionId::new("session-1").unwrap(),
        title: "Current work".into(),
        status: SessionStatus::Active,
        manager: Default::default(),
        threads: Vec::new(),
    }];

    let view = session_choices(&sessions, "session-1");
    let state = ListSelectionState::new(view.model);

    assert_eq!(state.title(), "Resume session");
    assert_eq!(state.tabs()[0].label(), "All (1)");
    assert_eq!(state.visible_items()[0].label(), "Current work ✓");
    assert_eq!(
        view.actions.values().next(),
        Some(&SessionSelectionAction::Resume {
            session_id: "session-1".into(),
        })
    );
    assert!(
        view.actions
            .contains_key(&ListSelectionItemId::new("session:session-1"))
    );
}

#[test]
fn resume_action_ids_do_not_change_when_sessions_are_reordered() {
    let session = |id: &str| Session {
        session_id: SessionId::new(id).unwrap(),
        title: id.into(),
        status: SessionStatus::Active,
        manager: Default::default(),
        threads: Vec::new(),
    };

    let first = session_choices(&[session("session-1"), session("session-2")], "");
    let reordered = session_choices(&[session("session-2"), session("session-1")], "");

    assert_eq!(
        first.actions, reordered.actions,
        "session actions must be keyed by the real session identity"
    );
}

#[test]
fn resume_picker_groups_statuses_and_activates_the_selected_session() {
    let sessions = vec![
        Session {
            session_id: SessionId::new("session-1").unwrap(),
            title: "Active work".into(),
            status: SessionStatus::Active,
            manager: Default::default(),
            threads: Vec::new(),
        },
        Session {
            session_id: SessionId::new("session-2").unwrap(),
            title: "Archived work".into(),
            status: SessionStatus::Archived,
            manager: Default::default(),
            threads: Vec::new(),
        },
    ];

    let view = session_choices(&sessions, "session-2");
    let mut state = ListSelectionState::new(view.model);

    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        ["All (2)", "Active (1)", "Archived (1)"]
    );
    assert_eq!(state.selected_item().unwrap().label(), "Archived work ✓");

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        ListSelectionInputOutcome::Consumed
    );
    assert_eq!(state.active_tab().label(), "Active (1)");
    state.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let ListSelectionInputOutcome::Activate(item_id) =
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

#[test]
fn resume_items_show_time_branch_count_and_total_token_size() {
    let mut session = Session {
        session_id: SessionId::new("session-sized").unwrap(),
        title: "Sized work".into(),
        status: SessionStatus::Active,
        manager: Default::default(),
        threads: vec![SessionThread {
            thread_id: ThreadId::new("thread-sized").unwrap(),
            title: "main".into(),
            created_at_unix_ms: 1,
            completed_turn_duration_ms: 0,
            active_turn_started_at_unix_ms: None,
            usage: Default::default(),
            parent_thread_id: None,
            forked_from_id: None,
            status: ThreadStatus::Active,
        }],
    };
    session.manager.status_changed_at_unix_ms = 10_000;
    session.threads[0].usage.input_tokens.reported = 1_200;
    session.threads[0].usage.output_tokens.reported = 300;

    let view = session_choices_at(&[session], "different", 70_000);
    let state = ListSelectionState::new(view.model);

    assert_eq!(
        state.visible_items()[0].description(),
        Some("1m 00s ago  ·  1 branch  ·  1.5K tokens  ·  active  ·  session-sized")
    );
    assert!(state.search().is_some());
}
