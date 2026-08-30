use super::*;
use crate::render::test_context;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionStatus;

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
    state.selected = Some(SessionId::new("completed").unwrap());
    assert!(state.toggle_selected_pin());

    let labels = manager_rows(&sessions, &state.pinned)
        .into_iter()
        .map(|row| match row {
            ManagerRow::Heading(label) => label.to_owned(),
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

    assert_eq!(state.selected().unwrap().as_str(), "question");
    assert!(state.select_next(&sessions));
    assert_eq!(state.selected().unwrap().as_str(), "working");
    assert!(state.select_next(&sessions));
    assert_eq!(state.selected().unwrap().as_str(), "completed");
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
        false,
        false,
        0,
        72_000,
        72,
        test_context(),
    ));

    assert!(text.starts_with("⠋ working"));
    assert!(text.contains("Running targeted tests"));
    assert!(text.ends_with("1m 02s"));
    assert_eq!(text.width(), 72);
}

#[test]
fn completed_time_is_relative_but_working_time_is_runtime() {
    let completed = session("done", SessionManagerStatus::Completed, None);
    let working = session("work", SessionManagerStatus::Working, None);

    assert_eq!(elapsed_label(&completed, 72_000), "1m 02s ago");
    assert_eq!(elapsed_label(&working, 72_000), "1m 02s");
}

#[test]
fn status_icons_have_distinct_semantics_and_working_animation_advances_on_tick() {
    let context = test_context();
    assert_eq!(
        status_icon(SessionManagerStatus::Failed, 0, context),
        ('●', context.danger())
    );
    assert_eq!(
        status_icon(SessionManagerStatus::Completed, 0, context),
        ('●', context.success())
    );
    assert_eq!(
        status_icon(SessionManagerStatus::Stopped, 0, context),
        ('■', context.muted())
    );

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
fn viewport_never_trades_the_selected_row_for_a_group_heading() {
    let sessions = vec![
        session("question", SessionManagerStatus::NeedsInput, None),
        session("working", SessionManagerStatus::Working, None),
        session("completed", SessionManagerStatus::Completed, None),
    ];
    let rows = manager_rows(&sessions, &BTreeSet::new());

    let start = visible_start(&rows, Some(5), 3);

    assert_eq!(start, 3);
    assert!(5 < start + 3);
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
