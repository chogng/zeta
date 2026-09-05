use super::*;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionStatus;

#[test]
fn archived_is_a_peer_heading_and_owns_archived_sessions_even_when_pinned() {
    let mut archived = session("old", SessionManagerStatus::Idle, None);
    archived.status = SessionStatus::Archived;
    let sessions = vec![
        session("current", SessionManagerStatus::Idle, None),
        archived,
    ];
    let mut state = SessionManagerState::default();
    state.pinned.insert(SessionId::new("old").unwrap());
    state.reconcile(&sessions);
    state.select_next(&sessions);
    assert!(state.archived_selected());
    assert!(state.selected_archive_ids(&sessions).is_empty());
    let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
    for expanded in [false, true] {
        state.set_archived_expanded(expanded);
        assert_eq!(
            state.selection_hint(),
            if expanded {
                "Enter to collapse · Esc to return to input"
            } else {
                "Enter to expand · Esc to return to input"
            }
        );
        terminal
            .draw(|frame| {
                draw_manager(
                    frame,
                    frame.area(),
                    state.view(&sessions),
                    None,
                    None,
                    crate::render::test_context(),
                )
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)].symbol(), "I");
        assert_eq!(buffer[(0, 2)].symbol(), "A");
        let heading = (0..40)
            .map(|column| buffer[(column, 2)].symbol())
            .collect::<String>();
        assert_eq!(heading.trim_end(), "Archived (1)");
        assert_eq!(buffer[(0, 0)].fg, buffer[(0, 2)].fg);
        assert_eq!(buffer[(0, 0)].modifier, buffer[(0, 2)].modifier);
        assert_eq!(
            pointer_target_at(Rect::new(0, 0, 40, 8), state.view(&sessions), 0, 2),
            Some(SessionManagerPointerTarget::Archived)
        );
        let rows = manager_rows(&sessions, &state.pinned, expanded);
        assert_eq!(rows.len(), if expanded { 4 } else { 3 });
    }
    state.select_next(&sessions);
    assert!(state.selected_is_archived());
    assert!(!state.toggle_selected_pin());
    state.reconcile(&sessions[..1]);
    assert!(state.archived_selected());
}

#[test]
fn groups_sessions_by_management_status_and_keeps_pinned_first() {
    let sessions = vec![
        session("completed", SessionManagerStatus::Completed, None),
        session(
            "working",
            SessionManagerStatus::Working,
            Some(SessionManagerActivity::Operation {
                text: "Running tests".into(),
            }),
        ),
        session(
            "question",
            SessionManagerStatus::NeedsInput,
            Some(SessionManagerActivity::Question {
                text: "Which API?".into(),
            }),
        ),
    ];
    let mut state = SessionManagerState::default();
    state.reconcile(&sessions);
    state.selected = Some(SessionManagerPointerTarget::Session(
        SessionId::new("completed").unwrap(),
    ));
    assert!(state.toggle_selected_pin());

    let labels = manager_rows(&sessions, &state.pinned, state.archived_expanded)
        .into_iter()
        .map(|row| match row {
            ManagerRow::Heading { group, .. } => group.label().to_owned(),
            ManagerRow::Session(session) => session.session_id.to_string(),
        })
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        [
            "Pinned",
            "completed",
            "Needs input",
            "question",
            "Working",
            "working",
            "Archived",
        ]
    );
}

#[test]
fn navigation_follows_the_visible_group_order() {
    let sessions = vec![
        session("completed", SessionManagerStatus::Completed, None),
        session("working", SessionManagerStatus::Working, None),
        session("question", SessionManagerStatus::NeedsInput, None),
    ];
    let mut state = SessionManagerState::default();
    state.reconcile(&sessions);

    assert_eq!(state.selected_session().unwrap().as_str(), "question");
    assert!(state.select_next(&sessions));
    assert_eq!(state.selected_session().unwrap().as_str(), "working");
    assert!(state.select_next(&sessions));
    assert_eq!(state.selected_session().unwrap().as_str(), "completed");
}

#[test]
fn row_starts_with_status_icon_and_keeps_name_activity_and_time_columns() {
    let session = session(
        "working",
        SessionManagerStatus::Working,
        Some(SessionManagerActivity::Operation {
            text: "Running targeted tests".into(),
        }),
    );
    let text = line_text(&session_line(
        &session,
        InteractionState::default(),
        0,
        7_210_000,
        72,
        crate::render::test_context(),
    ));

    assert!(text.starts_with("  ⠋ working"));
    assert!(text.contains("Running targeted tests"));
    assert!(text.ends_with("2h"));
    assert_eq!(text.width(), 72);
}

#[test]
fn completed_time_is_relative_but_working_time_is_runtime() {
    let completed = session("done", SessionManagerStatus::Completed, None);
    let working = session("work", SessionManagerStatus::Working, None);

    assert_eq!(elapsed_label(&completed, 1_810_000), "");
    assert_eq!(elapsed_label(&working, 10_750_000), "2h");
    assert_eq!(elapsed_label(&completed, 259_210_000), "3d ago");
}

#[test]
fn status_icons_have_distinct_semantics_and_working_animation_advances_on_tick() {
    assert_eq!(status_icon(SessionManagerStatus::Failed, 0), '●');
    assert_eq!(status_icon(SessionManagerStatus::Completed, 0), '●');
    assert_eq!(status_icon(SessionManagerStatus::Stopped, 0), '■');

    let sessions = vec![session("working", SessionManagerStatus::Working, None)];
    let mut state = SessionManagerState::default();
    let started = Instant::now();
    state.refresh_time(started, &sessions);
    let first = state.animation_frame;
    assert!(state.refresh_time(started + ANIMATION_INTERVAL, &sessions));
    assert_ne!(state.animation_frame, first);
}

#[test]
fn summary_column_stays_empty_without_a_configured_summary_result() {
    let session = session("ready", SessionManagerStatus::ReadyForReview, None);

    assert_eq!(activity_text(&session), "");
}

#[test]
fn viewport_reserves_rows_for_both_overflow_notices() {
    assert_eq!(
        manager_viewport(20, Some(10), 5),
        ManagerViewport { start: 8, end: 11 }
    );
}

#[test]
fn pointer_activation_does_not_take_keyboard_focus_or_selection() {
    let sessions = vec![session("idle", SessionManagerStatus::Idle, None)];
    let mut state = SessionManagerState::default();
    state.reconcile(&sessions);
    assert!(pointer_target_at(Rect::new(0, 0, 32, 5), state.view(&sessions), 2, 0).is_none());
    let target = pointer_target_at(Rect::new(0, 0, 32, 5), state.view(&sessions), 2, 1)
        .expect("the first session row is interactive");

    assert_eq!(
        target,
        SessionManagerPointerTarget::Session(SessionId::new("idle").unwrap())
    );
    let detail = state.details_selected(&sessions).unwrap();

    assert!(!state.focused());
    assert_eq!(state.selected_session().unwrap().as_str(), "idle");
    assert_eq!(detail.title(), "Session details");
}

#[test]
fn group_headings_are_static_and_all_sessions_remain_visible() {
    let sessions = (0..4)
        .map(|index| session(&format!("idle-{index}"), SessionManagerStatus::Idle, None))
        .collect::<Vec<_>>();
    let mut state = SessionManagerState::default();
    state.reconcile(&sessions);

    assert_eq!(state.selected_archive_ids(&sessions).len(), 1);
    assert_eq!(
        state.details_selected(&sessions).unwrap().title(),
        "Session details"
    );
    assert_eq!(
        manager_rows(&sessions, &state.pinned, state.archived_expanded).len(),
        6
    );
    assert_eq!(
        state.selection_hint(),
        "Enter to open · Space to preview · Ctrl+X to archive · i to details"
    );
}

#[test]
fn session_preview_is_read_only_manager_detail() {
    let mut session = session("idle", SessionManagerStatus::Idle, None);
    session.threads.push(zeta_protocol::SessionThread {
        thread_id: zeta_protocol::ThreadId::new("thread-idle").unwrap(),
        title: "main".into(),
        created_at_unix_ms: 0,
        completed_turn_duration_ms: 0,
        active_turn_started_at_unix_ms: None,
        usage: Default::default(),
        parent_thread_id: None,
        forked_from_id: None,
        status: zeta_protocol::ThreadStatus::Active,
    });
    session.threads[0].usage.input_tokens.reported = 1_200;
    session.threads[0].usage.output_tokens.reported = 300;
    let sessions = vec![session];
    let mut state = SessionManagerState::default();
    state.reconcile(&sessions);

    let detail = state.details_selected(&sessions).unwrap();

    assert_eq!(detail.title(), "Session details");
    assert!(
        detail
            .rows()
            .iter()
            .any(|row| row.label() == "Branches" && row.value() == "1 branch")
    );
    assert!(
        detail
            .rows()
            .iter()
            .any(|row| row.label() == "Size" && row.value() == "1.5K tokens")
    );
    assert!(
        detail
            .rows()
            .iter()
            .any(|row| row.label() == "Root" && row.value() == "main · active")
    );
}

#[test]
fn rendering_shows_group_count_overflow_and_high_contrast_selection() {
    let sessions = (0..8)
        .map(|index| session(&format!("idle-{index}"), SessionManagerStatus::Idle, None))
        .collect::<Vec<_>>();
    let mut state = SessionManagerState::default();
    state.reconcile(&sessions);
    state.focus();
    let backend = TestBackend::new(32, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            draw_manager(
                frame,
                Rect::new(0, 0, 32, 5),
                state.view(&sessions),
                None,
                None,
                crate::render::test_context(),
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rendered = (0..5)
        .map(|row| {
            (0..32)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("Idle (8)"));
    assert!(rendered.contains("more below"));
    let context = crate::render::test_context();
    assert_eq!(buffer[(0, 0)].fg, context.muted());
    assert_eq!(buffer[(0, 1)].symbol(), ">");
    assert_eq!(buffer[(0, 1)].fg, context.selection_foreground());
    assert_eq!(buffer[(0, 1)].bg, context.selection_background());

    for _ in 0..5 {
        state.select_next(&sessions);
    }
    terminal
        .draw(|frame| {
            draw_manager(
                frame,
                Rect::new(0, 0, 32, 5),
                state.view(&sessions),
                None,
                None,
                crate::render::test_context(),
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rendered = (0..5)
        .map(|row| {
            (0..32)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("more above"));
    assert!(rendered.contains("more below"));
}

#[test]
fn blurred_manager_keeps_its_cursor_without_rendering_keyboard_selection() {
    let sessions = vec![session("idle", SessionManagerStatus::Idle, None)];
    let mut state = SessionManagerState::default();
    state.reconcile(&sessions);
    let mut terminal = Terminal::new(TestBackend::new(32, 3)).unwrap();

    terminal
        .draw(|frame| {
            draw_manager(
                frame,
                frame.area(),
                state.view(&sessions),
                None,
                None,
                crate::render::test_context(),
            )
        })
        .unwrap();
    assert_eq!(
        terminal.backend().buffer()[(0, 0)].fg,
        crate::render::test_context().muted()
    );

    state.focus();
    terminal
        .draw(|frame| {
            draw_manager(
                frame,
                frame.area(),
                state.view(&sessions),
                None,
                None,
                crate::render::test_context(),
            )
        })
        .unwrap();
    assert_eq!(
        terminal.backend().buffer()[(0, 1)].bg,
        crate::render::test_context().selection_background()
    );
}

fn session(
    id: &str,
    status: SessionManagerStatus,
    activity: Option<SessionManagerActivity>,
) -> Session {
    Session {
        session_id: SessionId::new(id).unwrap(),
        title: id.into(),
        status: SessionStatus::Active,
        manager: SessionManagerInfo {
            status,
            status_changed_at_unix_ms: 10_000,
            activity,
            summary: None,
        },
        threads: Vec::new(),
    }
}

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.to_string())
        .collect()
}
