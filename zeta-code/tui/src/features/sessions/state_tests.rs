use super::SessionsState;
use super::TerminalScreen;
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
