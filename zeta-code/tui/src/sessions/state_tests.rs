use super::SessionManagerInputOutcome;
use super::SessionsState;
use super::TerminalScreen;
use crate::sessions::Command;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerStatus;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[test]
fn manager_is_directly_left_of_the_active_session() {
    let mut state = SessionsState::default();
    state.install_catalog(
        vec![session("one"), session("two")],
        session_id("one"),
        thread_id("one"),
    );

    assert_eq!(state.previous_screen(), Some(TerminalScreen::Manager));
    state.show_manager();
    assert_eq!(state.previous_screen(), None);
    assert_eq!(
        state.next_screen(),
        Some(TerminalScreen::Session(session_id("one")))
    );
}

#[test]
fn historical_sessions_are_not_horizontal_screens() {
    let mut completed = session("completed");
    completed.manager.status = SessionManagerStatus::Completed;
    let mut working = session("working");
    working.manager.status = SessionManagerStatus::Working;
    let mut question = session("question");
    question.manager.status = SessionManagerStatus::NeedsInput;
    let mut state = SessionsState::default();
    state.install_catalog(
        vec![completed, working, question],
        session_id("completed"),
        thread_id("completed"),
    );

    state.show_manager();
    assert_eq!(
        state.next_screen(),
        Some(TerminalScreen::Session(session_id("completed")))
    );
    state.show_session(session_id("working"), thread_id("working"));
    assert_eq!(state.previous_screen(), Some(TerminalScreen::Manager));
    assert_eq!(state.next_screen(), None);
    state.show_manager();
    assert_eq!(
        state.next_screen(),
        Some(TerminalScreen::Session(session_id("working")))
    );
}

#[test]
fn showing_a_session_clears_manager_focus() {
    let mut state = SessionsState::default();
    state.install_catalog(vec![session("one")], session_id("one"), thread_id("one"));
    state.show_manager();
    state.manager_mut().focus();

    state.show_session(session_id("one"), thread_id("one"));

    assert_eq!(
        state.screen(),
        Some(&TerminalScreen::Session(session_id("one")))
    );
    assert!(!state.manager().focused());
}

#[test]
fn each_session_remembers_its_last_viewed_thread() {
    let mut state = SessionsState::default();
    state.install_catalog(vec![session("one")], session_id("one"), thread_id("root"));
    state.remember_viewed_thread(session_id("one"), thread_id("child"));

    assert_eq!(
        state.remembered_thread(&session_id("one")),
        Some(&thread_id("child"))
    );
}

#[test]
fn reentering_a_session_falls_back_to_main_after_the_viewed_subagent_completes() {
    let mut catalog_session = session("one");
    catalog_session.threads = vec![
        SessionThread {
            thread_id: thread_id("one"),
            title: "main".into(),
            created_at_unix_ms: 1,
            completed_turn_duration_ms: 0,
            active_turn_started_at_unix_ms: None,
            usage: Default::default(),
            parent_thread_id: None,
            forked_from_id: None,
            status: ThreadStatus::Active,
        },
        SessionThread {
            thread_id: thread_id("child"),
            title: "child".into(),
            created_at_unix_ms: 2,
            completed_turn_duration_ms: 0,
            active_turn_started_at_unix_ms: None,
            usage: Default::default(),
            parent_thread_id: Some(thread_id("one")),
            forked_from_id: None,
            status: ThreadStatus::Archived,
        },
    ];
    let mut state = SessionsState::default();
    state.install_catalog(vec![catalog_session], session_id("one"), thread_id("child"));

    assert_eq!(
        state.restorable_thread(&session_id("one")),
        Some(thread_id("one"))
    );
}

#[test]
fn manager_input_requires_a_visible_focused_manager_and_resumes_the_remembered_thread() {
    let mut state = SessionsState::default();
    state.install_catalog(vec![session("one")], session_id("one"), thread_id("child"));
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    state.manager_mut().focus();
    assert!(matches!(
        state.handle_manager_key(enter),
        SessionManagerInputOutcome::Unhandled
    ));
    state.show_manager();
    state.manager_mut().blur();
    assert!(matches!(
        state.handle_manager_key(enter),
        SessionManagerInputOutcome::Unhandled
    ));
    state.manager_mut().focus();
    let SessionManagerInputOutcome::Command(command) = state.handle_manager_key(enter) else {
        panic!("focused manager must resume the selected session");
    };
    assert_eq!(
        command,
        Command::Resume {
            session_id: "one".into(),
            preferred_thread_id: Some(thread_id("child")),
        }
    );
    assert_eq!(state.screen(), Some(&TerminalScreen::Manager));
}

#[test]
fn manager_repeats_only_navigation_and_keeps_mutations_for_key_presses() {
    let mut state = SessionsState::default();
    state.install_catalog(vec![session("one")], session_id("one"), thread_id("one"));
    state.show_manager();
    state.manager_mut().focus();
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        for (code, modifiers) in [
            (KeyCode::Enter, KeyModifiers::NONE),
            (KeyCode::Char(' '), KeyModifiers::NONE),
            (KeyCode::Char('i'), KeyModifiers::NONE),
            (KeyCode::Char('p'), KeyModifiers::NONE),
            (KeyCode::Char('x'), KeyModifiers::CONTROL),
        ] {
            assert!(matches!(
                state.handle_manager_key(KeyEvent::new_with_kind(code, modifiers, kind)),
                SessionManagerInputOutcome::Unhandled
            ));
        }
    }
    assert!(state.preview.is_none());
    assert!(state.details.is_none());
    assert!(matches!(
        state.handle_manager_key(KeyEvent::new_with_kind(
            KeyCode::Home,
            KeyModifiers::NONE,
            KeyEventKind::Repeat
        )),
        SessionManagerInputOutcome::Consumed
    ));
    // Home selects the group heading; Down selects its first session.
    state.handle_manager_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let SessionManagerInputOutcome::Command(command) =
        state.handle_manager_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL))
    else {
        panic!("a key press must archive the selected active session");
    };
    assert_eq!(
        command,
        Command::Archive {
            session_ids: vec![session_id("one")]
        }
    );
}

fn session(value: &str) -> Session {
    Session {
        session_id: session_id(value),
        title: value.into(),
        status: SessionStatus::Active,
        manager: Default::default(),
        threads: Vec::new(),
    }
}

fn session_id(value: &str) -> SessionId {
    SessionId::new(value).unwrap()
}

fn thread_id(value: &str) -> ThreadId {
    ThreadId::new(value).unwrap()
}
