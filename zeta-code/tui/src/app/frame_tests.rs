use super::InputPointerTarget;
use super::draw;
use super::input_overlay_index_at;
use super::layout;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::AppEvent;
use crate::config::FollowUpMode;
use crate::config::TerminalSettings;
use crate::models::ModelSummary;
use crate::render::test_context;
use crate::status::RemainingContextWindow;
use crate::status::StatusViewData;
use crate::status::status_panel;
use crate::thread::TurnActivity;
use crate::thread::composer::ChatComposerPointerTarget;
use crate::thread::composer::ChatInputCatalog;
use crate::thread::composer::SkillCompletionItem;
use crate::thread::composer::SlashCommandCatalog;
use crate::thread::composer::built_in_slash_command_definitions;
use crate::thread::composer::file_search::FileSearchManager;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::search_box::SearchBoxModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use unicode_width::UnicodeWidthStr;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_protocol::ContentDigest;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerActivity;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionManagerStatus;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;
use zeta_slash_commands::SlashCommandArgumentMode;
use zeta_slash_commands::SlashCommandDefinition;

fn set_follow_up_mode(app: &mut App, mode: FollowUpMode) {
    let mut settings = TerminalSettings::default();
    settings.set_follow_up_mode(mode);
    app.update(AppEvent::ConfigSettingsReceived(settings));
}

#[test]
fn empty_frame_uses_lightweight_chrome_and_a_welcome_banner() {
    let rendered = render(&App::new(), 80, 20);

    assert!(!rendered.contains("dir assistant"));
    assert!(rendered.contains(concat!("Zeta Code v", env!("CARGO_PKG_VERSION"))));
    assert!(rendered.contains(" ▀▙▄▄▄▟▀"));
    assert!(rendered.contains("Automatic model · Access unknown"));
    assert!(!rendered.contains("enter send"));
    assert!(!rendered.contains("ctrl-v image"));
    let status_line = rendered.lines().last().unwrap();
    assert_eq!(status_line.trim_end(), "  ⏸ ask permissions on");
}

#[test]
fn top_tip_notice_uses_the_fixed_row_above_chat_input_without_changing_layout() {
    let mut app = App::new();
    let terminal_area = Rect::new(0, 0, 80, 20);
    let areas_before = layout(&app, terminal_area).session;

    app.update(AppEvent::TopTipNoticeShown(
        "Copied 246 chars to clipboard".into(),
    ));

    let areas_after = layout(&app, terminal_area).session;
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();
    let notice_row = usize::from(areas_after.top_tip.y);

    assert_eq!(areas_after, areas_before);
    assert_eq!(areas_after.top_tip.height, 1);
    assert_eq!(areas_after.top_tip.bottom(), areas_after.composer.y);
    assert!(
        rows[notice_row]
            .trim_end()
            .ends_with("Copied 246 chars to clipboard")
    );
    assert!(!rows[notice_row].contains("shift+tab"));
    assert!(!rows.last().unwrap().contains("Copied"));
}

#[test]
fn status_panel_expands_or_scrolls_with_available_height_and_escape_restores_chat_input() {
    let mut app = App::new();
    let terminal_area = Rect::new(0, 0, 80, 20);
    app.insert_text("/");
    assert!(app.completion().is_some());
    let before = layout(&app, terminal_area).session;

    let usage = zeta_protocol::ModelUsageSummary::default();
    let reference_cost = zeta_protocol::ModelReferenceCostSummary::default();
    app.update(AppEvent::StatusPanelOpened(status_panel(StatusViewData {
        model: "openai/gpt",
        full_context_window: Some(100_000),
        available_context_window: Some(90_000),
        remaining_context_window: RemainingContextWindow::Exact {
            remaining_tokens: 80_000,
            available_tokens: 90_000,
        },
        usage: &usage,
        reference_cost: &reference_cost,
        session_id: "session-1",
        thread_id: "thread-1",
        thread_sequence: 4,
    })));

    assert_eq!(
        layout(&app, Rect::new(0, 0, 80, 30))
            .session
            .composer
            .height,
        18
    );
    assert_eq!(layout(&app, terminal_area).session.composer.height, 13);
    assert_eq!(layout(&app, terminal_area).session.bottom.height, 2);
    assert_ne!(layout(&app, terminal_area).session, before);
    assert!(app.command_panel().is_some());
    assert!(app.overlay().is_none());
    assert!(app.completion().is_none());
    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    assert_eq!(app.input(), "/");
    let rendered = render(&app, 80, 20);
    let rows = rendered.lines().collect::<Vec<_>>();
    assert!(rows[18].trim().is_empty());
    assert_eq!(
        rows[19].trim_end(),
        "  ↑/↓ scroll · PgUp/PgDn page · Home/End jump · Esc close"
    );
    assert_snapshot!("status_panel_adaptive_height", rendered);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.command_panel().is_none());
    assert_eq!(layout(&app, terminal_area).session, before);
    assert_eq!(app.input(), "/");
}

#[test]
fn manager_keeps_overflow_text_out_of_the_fixed_top_tip_row() {
    let mut app = App::new();
    app.update(AppEvent::SessionCatalogReceived(
        (0..24)
            .map(|index| {
                manager_session(
                    &format!("session-{index}"),
                    SessionManagerStatus::Idle,
                    None,
                )
            })
            .collect(),
    ));
    app.insert_text("/sessions");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TopTipNoticeShown(
        "Copied 246 chars to clipboard".into(),
    ));

    let terminal_area = Rect::new(0, 0, 100, 20);
    let areas = layout(&app, terminal_area).session;
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();
    let notice_row = rows[usize::from(areas.top_tip.y)];
    let manager_last_row = rows[usize::from(areas.top_tip.y.saturating_sub(1))];

    assert!(manager_last_row.contains("more below"));
    assert!(!notice_row.contains("more below"));
    assert!(
        notice_row
            .trim_end()
            .ends_with("Copied 246 chars to clipboard")
    );
}

#[test]
fn empty_session_input_offers_manager_navigation() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );

    let terminal_area = Rect::new(0, 0, 80, 20);
    let top_tip_row = usize::from(layout(&app, terminal_area).session.top_tip.y);
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[top_tip_row].contains("← for agents"));
    assert!(!rows[top_tip_row].contains("shift+tab"));
    assert_eq!(rows[19].trim_end(), "  ⏸ ask permissions on");

    assert!(!app.handle_tick(Instant::now() + Duration::from_secs(10)));
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();
    assert!(rows[top_tip_row].contains("← for agents"));
    assert!(!rows[top_tip_row].contains("shift+tab"));

    app.insert_text("draft");
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();
    let status_line = rendered.lines().last().unwrap();

    assert!(!rows[top_tip_row].contains("← for agents"));
    assert!(!rows[top_tip_row].contains("shift+tab"));
    assert_eq!(status_line.trim_end(), "  ⏸ ask permissions on");
    assert!(!status_line.contains("← for agents"));
}

#[test]
fn narrow_session_keeps_manager_tip_above_input_and_status_below() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );

    let terminal_area = Rect::new(0, 0, 24, 20);
    let top_tip_row = usize::from(layout(&app, terminal_area).session.top_tip.y);
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[top_tip_row].contains("← for agents"));
    assert!(!rows[top_tip_row].contains("shift+tab"));
    assert_eq!(rows[19].trim_end(), "  ⏸ ask permissions on");
}

#[test]
fn left_from_a_session_opens_the_manager() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );

    assert!(render(&app, 80, 20).contains("← for agents"));
    assert!(
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
            .is_none()
    );
    assert!(app.session_manager_view().is_some());
    assert!(
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .is_none()
    );
    assert!(app.session_manager_view().is_none());
}

#[test]
fn manager_keeps_welcome_and_renders_grouped_three_column_status_rows() {
    let mut app = App::new();
    app.update(AppEvent::SessionCatalogReceived(vec![
        manager_session(
            "needs-input",
            SessionManagerStatus::NeedsInput,
            Some(SessionManagerActivity::Question {
                text: "Which API should I use?".into(),
            }),
        ),
        manager_session(
            "working",
            SessionManagerStatus::Working,
            Some(SessionManagerActivity::Operation {
                text: "Running targeted tests".into(),
            }),
        ),
        manager_session("done", SessionManagerStatus::Completed, None),
    ]));
    app.insert_text("/sessions");
    assert!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .is_none()
    );

    let rendered = render(&app, 100, 28);
    let needs_input = rendered
        .lines()
        .find(|line| line.contains("needs-input"))
        .unwrap();
    let working = rendered
        .lines()
        .find(|line| line.contains("Running targeted tests"))
        .unwrap();
    let completed = rendered.lines().find(|line| line.contains("done")).unwrap();

    assert!(rendered.contains(concat!("Zeta Code v", env!("CARGO_PKG_VERSION"))));
    assert!(rendered.contains("Needs input"));
    assert!(rendered.contains("Working"));
    assert!(rendered.contains("Completed"));
    assert!(needs_input.starts_with("  ? needs-input"));
    assert!(needs_input.contains("Which API should I use?"));
    assert!(working.starts_with("  ⠋ working"));
    assert!(completed.starts_with("  ● done"));
    assert_eq!(
        rendered.lines().last().unwrap().trim_end(),
        "  enter to return"
    );
}

#[test]
fn pending_steer_is_shown_once_in_chat_history() {
    let mut app = App::new();
    set_follow_up_mode(&mut app, FollowUpMode::Steer);
    app.insert_text("start");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("check the tests first");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let rendered = render(&app, 80, 20);

    assert!(!rendered.contains("Steer  1 sending"));
    assert_eq!(rendered.matches("check the tests first").count(), 1);
}

#[test]
fn status_line_uses_a_distinct_symbol_for_each_approval_mode() {
    let mut app = App::new();
    let ask_permissions = render(&app, 80, 20)
        .lines()
        .last()
        .unwrap()
        .trim_end()
        .to_owned();

    app.set_next_approval_mode(zeta_protocol::ApprovalMode::AutoReview);
    let auto_review = render(&app, 80, 20)
        .lines()
        .last()
        .unwrap()
        .trim_end()
        .to_owned();

    app.set_next_approval_mode(zeta_protocol::ApprovalMode::BypassPermissions);
    let bypass_permissions = render(&app, 80, 20)
        .lines()
        .last()
        .unwrap()
        .trim_end()
        .to_owned();

    assert_eq!(
        [ask_permissions, auto_review, bypass_permissions],
        [
            "  ⏸ ask permissions on",
            "  ⏩  auto review on",
            "  ▶ bypass permissions on",
        ]
    );
}

#[test]
fn turn_activity_does_not_enter_status_line() {
    let mut app = App::new();
    app.insert_text("start");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("change direction");

    let rendered = render(&app, 80, 20);

    assert!(!rendered.contains("working"));
}

#[test]
fn queued_message_is_visible_inline_and_counted_in_status_line() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("edit this later");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let rendered = render(&app, 80, 20);

    assert!(rendered.contains("Queue 1: edit this later"));
    assert!(
        rendered
            .lines()
            .any(|line| line.trim_start().starts_with("queue 1"))
    );
}

#[test]
fn follow_up_mode_keeps_queue_count_in_status_line() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    set_follow_up_mode(&mut app, FollowUpMode::Steer);
    app.insert_text("change direction");

    assert!(!render(&app, 80, 20).contains("working"));

    set_follow_up_mode(&mut app, FollowUpMode::Queue);
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    set_follow_up_mode(&mut app, FollowUpMode::Steer);

    assert!(
        render(&app, 80, 20)
            .lines()
            .any(|line| line.trim_start().starts_with("queue 1"))
    );
}

#[test]
fn status_line_uses_a_distinct_color_for_each_approval_mode_symbol() {
    let mut app = App::new();
    let ask_permissions = render_buffer(&app, 80, 20);
    assert_eq!(ask_permissions[(2, 19)].fg, test_context().warning());
    assert_eq!(
        ask_permissions[(2 + "⏸".width() as u16, 19)].fg,
        test_context().chat_input_chrome()
    );

    app.set_next_approval_mode(zeta_protocol::ApprovalMode::AutoReview);
    let auto_review = render_buffer(&app, 80, 20);
    assert_eq!(auto_review[(2, 19)].fg, test_context().accent());
    assert_eq!(
        auto_review[(2 + "⏩".width() as u16, 19)].fg,
        test_context().chat_input_chrome()
    );

    app.set_next_approval_mode(zeta_protocol::ApprovalMode::BypassPermissions);
    let bypass_permissions = render_buffer(&app, 80, 20);
    assert_eq!(bypass_permissions[(2, 19)].fg, test_context().danger());
    assert_eq!(
        bypass_permissions[(2 + "▶".width() as u16, 19)].fg,
        test_context().chat_input_chrome()
    );
}

#[test]
fn status_line_colors_current_and_next_modes_independently() {
    let mut app = App::new();
    app.set_current_approval_mode(Some(zeta_protocol::ApprovalMode::AskPermissions));
    app.set_next_approval_mode(zeta_protocol::ApprovalMode::AutoReview);

    let buffer = render_buffer(&app, 80, 20);
    let next_icon_column = 2 + "⏸ current: ask permissions on · ".width() as u16;
    assert_eq!(buffer[(2, 19)].fg, test_context().warning());
    assert_eq!(buffer[(next_icon_column, 19)].fg, test_context().accent());
}

#[test]
fn path_is_only_visible_in_the_empty_welcome_banner() {
    let mut app = App::for_dir(Path::new("/work/zeta"));

    let empty = render(&app, 80, 20);
    assert!(empty.contains("/work/zeta"));
    assert!(!empty.lines().last().unwrap().contains("/work/zeta"));

    app.update(AppEvent::ProductNotice("Conversation started.".into()));
    assert!(!render(&app, 80, 20).contains("/work/zeta"));
}

#[test]
fn status_line_renders_the_configured_model_without_provider() {
    let mut app = App::new();
    app.update(AppEvent::ModelSummaryReceived(ModelSummary::from_catalog(
        Some(ModelRefDto {
            provider: "anthropic".into(),
            model: "claude-sonnet".into(),
        }),
        None,
    )));

    let buffer = render_buffer(&app, 80, 20);
    let context_line = (0..80)
        .map(|x| buffer[(x, 18)].symbol())
        .collect::<String>();
    let policy_line = (0..80)
        .map(|x| buffer[(x, 19)].symbol())
        .collect::<String>();

    assert_eq!(context_line.trim_end(), "  claude-sonnet");
    assert_eq!(policy_line.trim_end(), "  ⏸ ask permissions on");
    assert_eq!(buffer[(2, 19)].fg, test_context().warning());
}

#[test]
fn narrow_status_line_keeps_the_first_configured_item() {
    let mut app = App::new();
    app.update(AppEvent::ModelSummaryReceived(ModelSummary::from_catalog(
        Some(ModelRefDto {
            provider: "anthropic".into(),
            model: "claude-sonnet".into(),
        }),
        None,
    )));

    let rendered = render(&app, 24, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert_eq!(rows[18].trim_end(), "  claude-sonnet");
    assert_eq!(rows[19].trim_end(), "  ⏸ ask permissions on");
}

#[test]
fn chat_input_uses_light_gray_edge_to_edge_horizontal_rules_and_prompt() {
    let app = App::new();
    let terminal_area = Rect::new(0, 0, 80, 20);
    let input = layout(&app, terminal_area).input;
    let buffer = render_buffer(&app, terminal_area.width, terminal_area.height);

    for y in [input.y, input.bottom() - 1] {
        assert_eq!(buffer[(0, y)].symbol(), "─");
        assert_eq!(buffer[(0, y)].fg, test_context().chat_input_chrome());
        assert_eq!(buffer[(79, y)].symbol(), "─");
        assert_eq!(buffer[(79, y)].fg, test_context().chat_input_chrome());
    }
    let content_row = input.y + 1;
    assert_eq!(buffer[(0, content_row)].symbol(), ">");
    assert_eq!(buffer[(0, content_row)].fg, test_context().foreground());
    assert_eq!(buffer[(79, content_row)].symbol(), " ");
}

#[test]
fn policy_tip_appears_after_first_submission_and_each_policy_change() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );
    app.update(AppEvent::ModelSummaryReceived(ModelSummary::from_catalog(
        Some(ModelRefDto {
            provider: "anthropic".into(),
            model: "claude-sonnet".into(),
        }),
        None,
    )));
    let terminal_area = Rect::new(0, 0, 80, 20);
    let areas = layout(&app, terminal_area).session;
    let composer = areas.composer;
    let top_tip_row = areas.top_tip.y;

    let before = render(&app, 80, 20);
    assert!(
        before
            .lines()
            .nth(usize::from(top_tip_row))
            .unwrap()
            .contains("← for agents")
    );

    app.insert_text("hello");
    assert!(matches!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::SubmitTurn { .. })
    ));

    let buffer = render_buffer(&app, 80, 20);
    let bottom_row = layout(&app, terminal_area).session.bottom.bottom() - 1;
    let hint_column = 78 - "shift+tab to cycle policy".width() as u16;
    let hint = &buffer[(hint_column, top_tip_row)];

    assert_eq!(hint.symbol(), "s");
    assert_eq!(hint.fg, test_context().muted());
    assert!(hint.modifier.contains(Modifier::ITALIC));
    assert_eq!(
        (0..80)
            .map(|x| buffer[(x, bottom_row)].symbol())
            .collect::<String>()
            .trim_end(),
        "  ⏸ ask permissions on"
    );
    assert_eq!(buffer[(0, composer.y)].symbol(), "─");
    assert_eq!(buffer[(79, composer.y)].symbol(), "─");

    let first_tip_expired = Instant::now() + Duration::from_secs(6);
    assert!(app.handle_tick(first_tip_expired));
    let after = render(&app, 80, 20);
    let after_tip = after.lines().nth(usize::from(top_tip_row)).unwrap();
    assert!(!after_tip.contains("← for agents"));
    assert!(!after_tip.contains("shift+tab"));

    let policy_changed = first_tip_expired + Duration::from_secs(1);
    app.cycle_next_approval_mode(policy_changed);
    assert_eq!(app.approval_mode(), zeta_protocol::ApprovalMode::AutoReview);
    let after_change = render(&app, 80, 20);
    assert!(
        after_change
            .lines()
            .nth(usize::from(top_tip_row))
            .unwrap()
            .contains("shift+tab to cycle policy")
    );

    assert!(!app.handle_tick(policy_changed + Duration::from_secs(4)));
    app.cycle_next_approval_mode(policy_changed + Duration::from_secs(4));
    assert!(!app.handle_tick(policy_changed + Duration::from_secs(5)));
    assert!(app.handle_tick(policy_changed + Duration::from_secs(9)));
    let after_refreshed_tip = render(&app, 80, 20);
    assert!(
        !after_refreshed_tip
            .lines()
            .nth(usize::from(top_tip_row))
            .unwrap()
            .contains("shift+tab")
    );
}

#[test]
fn policy_tip_does_not_replace_navigation_before_the_conversation_starts() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );

    app.cycle_next_approval_mode(Instant::now());

    let rendered = render(&app, 80, 20);
    assert!(rendered.contains("← for agents"));
    assert!(!rendered.contains("shift+tab to cycle policy"));
}

#[test]
fn agent_thread_switcher_starts_at_the_empty_input_cursor_column() {
    let mut app = App::new();
    let session_id = SessionId::new("root").unwrap();
    let root_id = ThreadId::new("root").unwrap();
    app.update(AppEvent::ThreadContextChanged {
        session_id: session_id.clone(),
        thread_id: root_id.clone(),
    });
    app.update(AppEvent::SessionCatalogReceived(vec![Session {
        session_id,
        title: "Session".into(),
        status: SessionStatus::Active,
        manager: Default::default(),
        threads: vec![
            SessionThread {
                thread_id: root_id.clone(),
                title: "Main".into(),
                created_at_unix_ms: 1,
                completed_turn_duration_ms: 1_000,
                active_turn_started_at_unix_ms: None,
                usage: Default::default(),
                parent_thread_id: None,
                forked_from_id: None,
                status: ThreadStatus::Active,
            },
            SessionThread {
                thread_id: ThreadId::new("child").unwrap(),
                title: "Child".into(),
                created_at_unix_ms: 2,
                completed_turn_duration_ms: 2_000,
                active_turn_started_at_unix_ms: None,
                usage: Default::default(),
                parent_thread_id: Some(root_id),
                forked_from_id: None,
                status: ThreadStatus::Active,
            },
        ],
    }]));

    let rendered = render(&app, 40, 20);
    let main = rendered.lines().find(|line| line.contains("main")).unwrap();

    assert!(main.starts_with("  ● main"));
}

#[test]
fn multiline_chat_input_grows_upward_and_keeps_all_lines_visible() {
    let mut app = App::new();
    app.insert_text("first\nsecond\nthird");

    let terminal_area = Rect::new(0, 0, 80, 20);
    let input = layout(&app, terminal_area).input;
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[usize::from(input.y + 1)].contains("first"));
    assert!(rows[usize::from(input.y + 2)].contains("second"));
    assert!(rows[usize::from(input.y + 3)].contains("third"));
    assert_eq!(rows[19].trim_end(), "  ⏸ ask permissions on");
}

#[test]
fn turn_activity_keeps_permission_status_free_of_submission_hints() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));

    let rendered = render(&app, 80, 20);
    let status_line = rendered.lines().last().unwrap();

    assert_eq!(status_line.trim_end(), "  ⏸ ask permissions on");
    assert!(!status_line.contains("enter queue"));
    assert!(!status_line.contains("ctrl-c interrupt"));
}

#[test]
fn chat_input_soft_wraps_long_lines_instead_of_clipping_them() {
    let mut app = App::new();
    app.insert_text("abcdefghij");

    let terminal_area = Rect::new(0, 0, 8, 20);
    let input = layout(&app, terminal_area).input;
    let rendered = render(&app, terminal_area.width, terminal_area.height);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[usize::from(input.y + 1)].contains("abcdef"));
    assert!(rows[usize::from(input.y + 2)].contains("ghij"));
}

#[test]
fn command_panel_without_actions_keeps_the_two_bottom_rows_empty() {
    let mut app = App::new();
    app.update(AppEvent::ProductNotice(
        "Conversation remains visible.".into(),
    ));
    app.update(AppEvent::HelpOpened(help_view()));

    let rendered = render(&app, 80, 24);

    assert!(rendered.contains("Conversation remains visible."));
    assert!(rendered.contains("Help"));
    assert!(rendered.contains("Commands"));
    assert!(rendered.contains("Keys"));
    assert!(rendered.contains("Search commands and shortcuts"));
    assert!(!rendered.contains("Esc to close"));
    assert!(!rendered.contains("←/→/Tab to switch"));
    assert!(!rendered.contains("enter send"));
    assert!(!rendered.contains("ask permissions on"));
    let layout = super::layout(&app, Rect::new(0, 0, 80, 24));
    assert!(layout.input.is_empty());
    assert_eq!(layout.session.bottom.height, 2);
    let rows = rendered.lines().collect::<Vec<_>>();
    assert!(rows[22].trim().is_empty());
    assert!(rows[23].trim().is_empty());
    assert_eq!(layout.session.composer.bottom(), layout.session.bottom.y);
}

#[test]
fn command_panel_supports_keyboard_tab_switching_and_search() {
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(help_view()));

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

    let rendered = render(&app, 80, 24);
    assert!(rendered.contains("Esc"));
    assert!(rendered.find("Esc") < rendered.find("move selection"));

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.list_selection().is_none());
}

#[test]
fn theme_candidate_focus_repaints_only_the_command_panel_focus_border() {
    let mut app = App::new();
    app.update(AppEvent::HelpOpened(ListSelectionModel::new(
        "Theme",
        vec![ListSelectionGroup::new(
            "Themes",
            vec![
                ListSelectionItem::new("First")
                    .with_selection_foreground(Color::LightRed)
                    .with_presentation_focus(Color::Red),
                ListSelectionItem::new("Second")
                    .with_selection_foreground(Color::LightGreen)
                    .with_presentation_focus(Color::Green),
            ],
        )],
    )));

    let first = render_buffer(&app, 80, 24);
    let interaction_y = super::layout(&app, Rect::new(0, 0, 80, 24))
        .session
        .composer
        .y;
    assert_eq!(first[(4, 1)].fg, Color::Rgb(0x40, 0x85, 0xac));
    assert_eq!(first[(0, interaction_y)].fg, Color::Red);
    assert_eq!(
        first[(1, interaction_y)].fg,
        test_context().accent_surface_foreground()
    );
    assert_eq!(
        first[(1, interaction_y)].bg,
        test_context().accent_surface_background()
    );

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let second = render_buffer(&app, 80, 24);
    assert_eq!(second[(4, 1)].fg, first[(4, 1)].fg);
    assert_eq!(second[(0, interaction_y)].fg, Color::Green);
    assert_eq!(
        second[(1, interaction_y)].fg,
        test_context().accent_surface_foreground()
    );
    assert_eq!(
        second[(1, interaction_y)].bg,
        test_context().accent_surface_background()
    );
}

#[test]
fn error_detail_is_rendered_once_and_status_line_only_offers_recovery() {
    let mut app = App::new();
    app.update(AppEvent::FailureReported(
        "The configured model is unavailable.".into(),
    ));

    let rendered = render(&app, 80, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert_eq!(
        rendered
            .matches("The configured model is unavailable.")
            .count(),
        1
    );
    assert!(rendered.contains("ask permissions on"));
    assert!(!rows.iter().any(|line| line.trim() == "error"));
    assert_eq!(rows[19].trim_end(), "  ⏸ ask permissions on");
    assert!(!rendered.contains("ready to retry"));
    assert!(!rendered.contains("esc esc rewind"));
    assert!(!rendered.contains("StableTurnError"));
}

#[test]
fn submitted_slash_command_is_immediately_visible_in_the_transcript() {
    let mut app = App::new();
    app.insert_text("/status");

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let rendered = render(&app, 80, 20);
    assert!(rendered.lines().next().unwrap().contains("> /status"));
}

#[test]
fn command_completion_renders_an_adjacent_result_line() {
    let mut app = App::new();
    app.update(AppEvent::CommandCompleted {
        command: "/theme zeta-code-light".into(),
        result: "Theme set to Zeta Code Light".into(),
    });

    let rendered = render(&app, 80, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[0].contains("> /theme zeta-code-light"));
    assert!(rows[1].contains("└─ Theme set to Zeta Code Light"));
    assert!(rows[2].trim().is_empty());
}

#[test]
fn transcript_and_chat_input_content_start_in_the_same_column() {
    let mut app = App::new();
    app.update(AppEvent::CommandCompleted {
        command: "/theme zeta-code-light".into(),
        result: "Theme set to Zeta Code Light".into(),
    });
    app.insert_text("draft");

    let terminal_area = Rect::new(0, 0, 80, 20);
    let input = layout(&app, terminal_area).input;
    let buffer = render_buffer(&app, terminal_area.width, terminal_area.height);

    assert_eq!(buffer[(2, 0)].symbol(), "/");
    assert_eq!(buffer[(2, input.y + 1)].symbol(), "d");
}

#[test]
fn bare_slash_renders_the_first_command_window() {
    let mut app = App::new();
    app.insert_text("/");

    let rendered = render(&app, 80, 20);

    assert!(rendered.contains("/status"));
    assert!(rendered.contains("/statusline"));
    assert!(rendered.contains("/skills"));
    assert!(rendered.contains("/mcp"));
    assert!(rendered.contains("/resume"));
    assert!(rendered.contains("/archive"));
    assert!(!rendered.contains("/archive-thread"));
    assert!(!rendered.contains("/archive-session"));
    assert!(!rendered.contains("/thread "));
    assert!(!rendered.contains("/login"));
    assert!(!rendered.contains("/plugins"));
}

#[test]
fn slash_popup_uses_focus_colored_text_without_a_selection_surface() {
    let mut app = App::new();
    app.insert_text("/");

    let terminal_area = Rect::new(0, 0, 80, 20);
    let popup_top = layout(&app, terminal_area).input.y - 6;
    let buffer = render_buffer(&app, terminal_area.width, terminal_area.height);
    let selected = &buffer[(2, popup_top)];
    let unselected = &buffer[(2, popup_top + 1)];
    let surface_background = buffer[(0, 0)].bg;

    assert_eq!(selected.fg, test_context().focus());
    assert_eq!(selected.bg, surface_background);
    assert_eq!(selected.symbol(), "/");
    assert!(!selected.modifier.contains(Modifier::BOLD));
    assert_eq!(unselected.fg, test_context().muted());
    assert_eq!(unselected.bg, surface_background);
    assert_eq!(unselected.symbol(), "/");
    assert!(!unselected.modifier.contains(Modifier::BOLD));

    app.update_pointer_hover(Some(InputPointerTarget::Composer(
        ChatComposerPointerTarget::CompletionItem(2),
    )));
    let hovered_buffer = render_buffer(&app, terminal_area.width, terminal_area.height);
    let hovered = &hovered_buffer[(2, popup_top + 2)];
    assert_eq!(hovered.fg, test_context().focus());
    assert_eq!(hovered.bg, surface_background);
    assert!(!hovered.modifier.contains(Modifier::BOLD));
}

#[test]
fn slash_popup_hit_testing_maps_visible_rows_and_rejects_outside_clicks() {
    let mut app = App::new();
    app.insert_text("/");
    let terminal_area = Rect::new(0, 0, 80, 20);
    let popup_bottom = layout(&app, terminal_area).input.y;
    let popup_top = popup_bottom - 6;

    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_top),
        Some(0)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 77, popup_bottom - 1),
        Some(5)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 1, popup_top),
        None
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_bottom),
        None
    );

    for _ in 0..7 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_top),
        Some(2)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_bottom - 1),
        Some(7)
    );
}

#[test]
fn slash_popup_wraps_descriptions_to_two_clickable_lines_and_truncates_the_rest() {
    let slash_commands = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        [SlashCommandDefinition {
            name: "diagnose".into(),
            description: "one two three four five six seven eight nine ten".into(),
            argument_mode: SlashCommandArgumentMode::Optional,
        }],
    )
    .unwrap();
    let mut app = App::for_dir_with_slash_commands(Path::new("."), slash_commands);
    app.insert_text("/diagnose");
    let terminal_area = Rect::new(0, 0, 50, 20);
    let popup_bottom = layout(&app, terminal_area).input.y;
    let popup_top = popup_bottom - 2;

    let rendered = render(&app, 50, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[usize::from(popup_top)].contains("one two three four"));
    assert!(rows[usize::from(popup_top + 1)].contains("five six seven"));
    assert!(!rendered.contains("eight"));
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 29, popup_top),
        Some(0)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 29, popup_top + 1),
        Some(0)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 29, popup_top - 1),
        None
    );
}

#[test]
fn skill_popup_wraps_descriptions_to_two_clickable_lines_and_truncates_the_rest() {
    let slash_commands = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        std::iter::empty(),
    )
    .unwrap();
    let skill = SkillRef::pinned(
        SkillId::new(
            SkillSourceId::new("user:skill-source:test").unwrap(),
            SkillName::new("diagnose").unwrap(),
        ),
        ContentDigest::sha256(b"diagnose skill"),
    );
    let mut app = App::for_dir_with_slash_commands(Path::new("."), slash_commands.clone());
    app.replace_chat_input_catalog(ChatInputCatalog::new(
        slash_commands,
        vec![SkillCompletionItem::new(
            "diagnose".into(),
            "one two three four five six seven eight nine ten".into(),
            skill,
        )],
        Vec::new(),
    ));
    app.insert_text("$diagnose");
    let terminal_area = Rect::new(0, 0, 36, 20);
    let popup_bottom = layout(&app, terminal_area).input.y;
    let popup_top = popup_bottom - 2;

    let rendered = render(&app, 36, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[usize::from(popup_top)].contains("one two three four"));
    assert!(rows[usize::from(popup_top + 1)].contains("five six seven eight"));
    assert!(!rendered.contains("nine"));
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_top),
        Some(0)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_top + 1),
        Some(0)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_top - 1),
        None
    );
}

#[test]
fn empty_slash_popup_has_no_clickable_command_rows() {
    let mut app = App::new();
    app.insert_text("/unknown");

    assert_eq!(
        input_overlay_index_at(&app, Rect::new(0, 0, 80, 20), 2, 15),
        None
    );
}

#[test]
fn slash_query_filters_the_rendered_commands() {
    let mut app = App::new();
    app.insert_text("/q");

    let rendered = render(&app, 80, 20);

    assert!(rendered.contains("/quit"));
    assert!(!rendered.contains("/exit"));
}

#[test]
fn unmatched_slash_query_keeps_a_visible_empty_popup() {
    let mut app = App::new();
    app.insert_text("/unknown");

    let rendered = render(&app, 80, 20);

    assert!(rendered.contains("No matching commands"));
}

#[test]
fn escape_dismisses_the_slash_popup_without_clearing_input() {
    let mut app = App::new();
    app.insert_text("/");

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    let rendered = render(&app, 80, 20);
    assert!(!rendered.contains("/quit"));
    assert!(!rendered.contains("/exit"));
    assert_eq!(app.input(), "/");
}

#[test]
fn mention_popup_aligns_markers_with_the_query_and_highlights_fuzzy_matches() {
    let dir = std::env::temp_dir().join(format!(
        "zeta-tui-render-mention-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("docs/src-notes.md"), "notes").unwrap();
    fs::write(dir.join("src/lib.rs"), "lib").unwrap();
    let mut app = App::for_dir(&dir);
    app.insert_text("@src");
    wait_for_mention_results(&mut app, &dir);
    let terminal_area = Rect::new(0, 0, 80, 20);

    let buffer = render_buffer(&app, 80, 20);
    let Some(crate::thread::composer::CompletionView::Mention(popup)) = app.completion() else {
        panic!("expected mention suggestions");
    };
    let popup_top = layout(&app, terminal_area)
        .input
        .y
        .saturating_sub(popup.matches.len().min(2) as u16);
    for (row, matched) in popup.matches.iter().take(2).enumerate() {
        let screen_row = popup_top + row as u16;
        assert_eq!(buffer[(2, screen_row)].symbol(), "+");
        for (column, character) in matched.label.chars().enumerate() {
            assert_eq!(
                buffer[(column as u16 + 4, screen_row)].symbol(),
                character.to_string()
            );
        }
    }
    assert_eq!(
        buffer[(2, layout(&app, terminal_area).input.y + 1)].symbol(),
        "@"
    );
    let second = &popup.matches[1];
    let matched_index = second.indices[0];
    let unmatched_index = (0..second.label.chars().count())
        .find(|index| !second.indices.contains(index))
        .unwrap();
    assert!(
        buffer[(matched_index as u16 + 4, popup_top + 1)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(
        buffer[(matched_index as u16 + 4, popup_top + 1)].fg,
        test_context().foreground()
    );
    assert!(
        !buffer[(unmatched_index as u16 + 4, popup_top + 1)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(
        buffer[(unmatched_index as u16 + 4, popup_top + 1)].fg,
        test_context().muted()
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_top),
        Some(0)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 2, popup_top + 1),
        Some(1)
    );
    assert_eq!(
        input_overlay_index_at(&app, terminal_area, 1, popup_top + 1),
        None
    );
    let _ = fs::remove_dir_all(dir);
}

fn manager_session(
    id: &str,
    status: SessionManagerStatus,
    activity: Option<SessionManagerActivity>,
) -> Session {
    let session_id = SessionId::new(id).unwrap();
    Session {
        session_id: session_id.clone(),
        title: id.into(),
        status: if status == SessionManagerStatus::Completed {
            SessionStatus::Archived
        } else {
            SessionStatus::Active
        },
        manager: SessionManagerInfo {
            status,
            status_changed_at_unix_ms: current_unix_millis().saturating_sub(5_000),
            activity,
            summary: None,
        },
        threads: vec![SessionThread {
            thread_id: ThreadId::new(id).unwrap(),
            title: "main".into(),
            created_at_unix_ms: 0,
            completed_turn_duration_ms: 0,
            active_turn_started_at_unix_ms: None,
            usage: Default::default(),
            parent_thread_id: None,
            forked_from_id: None,
            status: if status == SessionManagerStatus::Completed {
                ThreadStatus::Archived
            } else {
                ThreadStatus::Active
            },
        }],
    }
}

fn enter_session(app: &mut App, id: &str, catalog: Vec<Session>) {
    app.update(AppEvent::ThreadContextChanged {
        session_id: SessionId::new(id).unwrap(),
        thread_id: ThreadId::new(id).unwrap(),
    });
    app.update(AppEvent::SessionCatalogReceived(catalog));
}

fn current_unix_millis() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

fn render(app: &App, width: u16, height: u16) -> String {
    let buffer = render_buffer(app, width, height);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn help_view() -> ListSelectionModel {
    ListSelectionModel::new(
        "Help",
        vec![
            ListSelectionGroup::new(
                "Commands",
                vec![
                    ListSelectionItem::new("/status").with_description("show status"),
                    ListSelectionItem::new("/model").with_description("show model"),
                ],
            ),
            ListSelectionGroup::new(
                "Keys",
                vec![
                    ListSelectionItem::new("↑ / ↓").with_description("move selection"),
                    ListSelectionItem::new("Esc").with_description("return to chat_input"),
                ],
            ),
        ],
    )
    .with_search(SearchBoxModel::new("Search commands and shortcuts"))
}

fn wait_for_mention_results(app: &mut App, dir: &Path) {
    let mut file_search = FileSearchManager::new(dir.to_path_buf());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(query) = app.mention_query() {
            file_search.update_query(query);
        } else {
            file_search.stop();
        }
        for snapshot in file_search.poll() {
            app.update(AppEvent::FileSearchSnapshotReceived(snapshot));
        }
        if matches!(
            app.completion(),
            Some(crate::thread::composer::CompletionView::Mention(popup))
                if popup.matches.len() >= 2
        ) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for mention render results"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn render_buffer(app: &App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(frame, app)).unwrap();
    terminal.backend().buffer().clone()
}
