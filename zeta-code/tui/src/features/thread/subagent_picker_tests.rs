use super::SubagentRegionState;
use super::draw_subagent_picker;
use crate::render::test_context;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[test]
fn completed_subagents_disappear_without_changing_stable_selection() {
    let mut session = session();
    let mut region = SubagentRegionState::default();
    let selected = thread_id("child-b");
    region.reconcile(Some(&session), Some(&selected));
    region.focus();

    session.threads[1].status = ThreadStatus::Archived;
    region.reconcile(Some(&session), Some(&selected));

    assert_eq!(region.selected(), Some(&selected));
    assert_eq!(region.view().rows.len(), 2);
}

#[test]
fn selection_drives_a_bounded_viewport() {
    let mut session = session();
    for index in 0..6 {
        session.threads.push(child(&format!("extra-{index}")));
    }
    let mut region = SubagentRegionState::default();
    region.reconcile(Some(&session), Some(&thread_id("root")));
    region.focus();
    for _ in 0..6 {
        region.select_next();
    }

    assert_eq!(region.view().rows.len(), 4);
    assert!(
        region
            .view()
            .rows
            .iter()
            .any(|row| Some(&row.thread_id) == region.selected())
    );
}

#[test]
fn rows_use_selection_dots_lowercase_names_and_right_aligned_elapsed_time() {
    let mut session = session();
    session.threads[1].title = "Review Agent".into();
    session.threads[0].completed_turn_duration_ms = 61_000;
    session.threads[1].completed_turn_duration_ms = 21_000;
    session.threads[1].active_turn_started_at_unix_ms = Some(52_000);
    let mut region = SubagentRegionState::default();
    region.reconcile(Some(&session), Some(&thread_id("child-a")));
    region.now_unix_ms = 62_000;
    let backend = TestBackend::new(30, 2);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| draw_subagent_picker(frame, frame.area(), region.view(), test_context()))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rows = (0..2)
        .map(|row| {
            (0..30)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(rows[0].trim_end(), "○ main                  1m 01s");
    assert_eq!(rows[1].trim_end(), "● review agent             31s");
    assert!(!rows.join("\n").contains("child-a"));
    assert_eq!(buffer[(0, 0)].fg, test_context().muted());
    assert_eq!(buffer[(0, 1)].fg, test_context().foreground());
}

#[test]
fn viewed_thread_and_focused_cursor_keep_separate_visual_identities() {
    let session = session();
    let mut region = SubagentRegionState::default();
    region.reconcile(Some(&session), Some(&thread_id("child-a")));
    region.focus();
    region.select_next();
    let mut terminal = Terminal::new(TestBackend::new(30, 3)).unwrap();

    terminal
        .draw(|frame| draw_subagent_picker(frame, frame.area(), region.view(), test_context()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 1)].symbol(), "●");
    assert_eq!(
        buffer[(0, 1)].bg,
        test_context().accent_surface_background()
    );
    assert_eq!(buffer[(0, 2)].symbol(), "○");
    assert_eq!(buffer[(0, 2)].bg, test_context().selection_background());

    region.blur();
    terminal
        .draw(|frame| draw_subagent_picker(frame, frame.area(), region.view(), test_context()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(0, 1)].bg,
        test_context().accent_surface_background()
    );
    assert_eq!(buffer[(0, 2)].fg, test_context().muted());
}

#[test]
fn region_is_absent_without_an_active_subagent() {
    let mut session = session();
    session.threads.truncate(1);
    let mut region = SubagentRegionState::default();

    region.reconcile(Some(&session), Some(&thread_id("root")));

    assert_eq!(region.desired_rows(), 0);
    assert!(region.view().rows.is_empty());
    assert!(!region.focus());
}

fn session() -> Session {
    Session {
        session_id: session_id("root"),
        title: "Session".into(),
        status: SessionStatus::Active,
        manager: Default::default(),
        threads: vec![root(), child("child-a"), child("child-b")],
    }
}

fn root() -> SessionThread {
    SessionThread {
        thread_id: thread_id("root"),
        title: "Main Task".into(),
        created_at_unix_ms: 1_000,
        completed_turn_duration_ms: 0,
        active_turn_started_at_unix_ms: None,
        usage: Default::default(),
        parent_thread_id: None,
        forked_from_id: None,
        status: ThreadStatus::Active,
    }
}

fn child(value: &str) -> SessionThread {
    SessionThread {
        thread_id: thread_id(value),
        title: value.into(),
        created_at_unix_ms: 1_000,
        completed_turn_duration_ms: 0,
        active_turn_started_at_unix_ms: None,
        usage: Default::default(),
        parent_thread_id: Some(thread_id("root")),
        forked_from_id: None,
        status: ThreadStatus::Active,
    }
}

fn session_id(value: &str) -> SessionId {
    SessionId::new(value).unwrap()
}

fn thread_id(value: &str) -> ThreadId {
    ThreadId::new(value).unwrap()
}
