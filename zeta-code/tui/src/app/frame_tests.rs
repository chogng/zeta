use super::draw;
use super::input_overlay_index_at;
use crate::app::App;
use crate::app::AppEvent;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use crate::features::config::FollowUpMode;
use crate::features::config::TerminalSettings;
use crate::features::file_search::FileSearchManager;
use crate::features::thread::TurnActivity;
use crate::render::test_context;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
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
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerActivity;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionManagerStatus;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

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
    assert!(rendered.contains("Welcome back!"));
    assert!(rendered.contains("Tips for getting started"));
    assert!(rendered.contains("Try asking"));
    assert!(!rendered.contains("enter send"));
    assert!(!rendered.contains("ctrl-v image"));
    let status_line = rendered.lines().last().unwrap();
    assert_eq!(status_line.trim_end(), "  ⏸ ask permissions on");
}

#[test]
fn empty_session_input_shows_agents_navigation_with_the_status_line() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );

    let rendered = render(&app, 80, 20);
    let status_line = rendered.lines().last().unwrap();

    assert!(status_line.starts_with("  ⏸ ask permissions on"));
    assert!(status_line.trim_end().ends_with("← agents"));

    app.insert_text("draft");
    let rendered = render(&app, 80, 20);
    let status_line = rendered.lines().last().unwrap();

    assert_eq!(status_line.trim_end(), "  ⏸ ask permissions on");
    assert!(!status_line.contains("← agents"));
}

#[test]
fn narrow_session_footer_keeps_status_and_agents_hint_on_separate_rows() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );

    let rendered = render(&app, 24, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert_eq!(rows[18].trim_end(), "  ⏸ ask permissions on");
    assert!(rows[19].trim_end().ends_with("← agents"));
}

#[test]
fn agents_navigation_hint_describes_the_left_key_that_opens_the_manager() {
    let mut app = App::new();
    enter_session(
        &mut app,
        "current",
        vec![manager_session("current", SessionManagerStatus::Idle, None)],
    );

    assert!(render(&app, 80, 20).contains("← agents"));
    assert!(
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
            .is_none()
    );
    assert!(app.session_manager_view().is_some());
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

    assert!(rendered.contains("Welcome back!"));
    assert!(rendered.contains("Needs input"));
    assert!(rendered.contains("Working"));
    assert!(rendered.contains("Completed"));
    assert!(needs_input.starts_with("? needs-input"));
    assert!(needs_input.contains("Which API should I use?"));
    assert!(working.starts_with("⠋ working"));
    assert!(completed.starts_with("● done"));
    assert_eq!(
        rendered.lines().last().unwrap().trim_end(),
        "  ↑ sessions · enter create"
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
fn working_draft_keeps_runtime_state_in_status_line() {
    let mut app = App::new();
    app.insert_text("start");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    app.insert_text("change direction");

    let rendered = render(&app, 80, 20);

    assert!(rendered.lines().any(|line| line.trim_end() == "  working"));
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
            .any(|line| line.trim_end() == "  working · queue 1")
    );
}

#[test]
fn follow_up_mode_does_not_replace_runtime_status_line() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));
    set_follow_up_mode(&mut app, FollowUpMode::Steer);
    app.insert_text("change direction");

    assert!(
        render(&app, 80, 20)
            .lines()
            .any(|line| line.trim_end() == "  working")
    );

    set_follow_up_mode(&mut app, FollowUpMode::Queue);
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    set_follow_up_mode(&mut app, FollowUpMode::Steer);

    assert!(
        render(&app, 80, 20)
            .lines()
            .any(|line| line.trim_end() == "  working · queue 1")
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
fn status_line_renders_the_configured_model() {
    let mut app = App::new();
    app.update(AppEvent::PreferredModelReceived(Some(ModelRefDto {
        provider: "anthropic".into(),
        model: "claude-sonnet".into(),
    })));

    let buffer = render_buffer(&app, 80, 20);
    let status_line = (0..80)
        .map(|x| buffer[(x, 19)].symbol())
        .collect::<String>();

    assert!(status_line.starts_with("  ⏸ ask permissions on"));
    assert!(status_line.trim_end().ends_with("anthropic/claude-sonnet"));
    assert_eq!(buffer[(2, 19)].fg, test_context().warning());
}

#[test]
fn narrow_status_line_keeps_the_first_configured_item() {
    let mut app = App::new();
    app.update(AppEvent::PreferredModelReceived(Some(ModelRefDto {
        provider: "anthropic".into(),
        model: "claude-sonnet".into(),
    })));

    let rendered = render(&app, 24, 20);
    let status_line = rendered.lines().last().unwrap();

    assert_eq!(status_line.trim_end(), "  ⏸ ask permissions on");
    assert!(!status_line.contains("claude"));
}

#[test]
fn chat_input_uses_light_gray_edge_to_edge_horizontal_rules_and_prompt() {
    let buffer = render_buffer(&App::new(), 80, 20);

    for y in [16, 18] {
        assert_eq!(buffer[(0, y)].symbol(), "─");
        assert_eq!(buffer[(0, y)].fg, test_context().chat_input_chrome());
        assert_eq!(buffer[(79, y)].symbol(), "─");
        assert_eq!(buffer[(79, y)].fg, test_context().chat_input_chrome());
    }
    assert_eq!(buffer[(0, 17)].symbol(), "❯");
    assert_eq!(buffer[(0, 17)].fg, test_context().chat_input_chrome());
    assert_eq!(buffer[(79, 17)].symbol(), " ");
}

#[test]
fn subagent_pane_starts_at_the_empty_input_cursor_column() {
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

    let rendered = render(&app, 80, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[15].contains("first"));
    assert!(rows[16].contains("second"));
    assert!(rows[17].contains("third"));
    assert_eq!(rows[19].trim_end(), "  ⏸ ask permissions on");
}

#[test]
fn working_status_line_keeps_configured_context_separate_from_runtime_text() {
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

    let rendered = render(&app, 8, 20);
    let rows = rendered.lines().collect::<Vec<_>>();

    assert!(rows[16].contains("abcdef"));
    assert!(rows[17].contains("ghij"));
}

#[test]
fn list_selection_pane_stacks_above_chat_input_and_keeps_history_visible() {
    let mut app = App::new();
    app.update(AppEvent::ProductNotice(
        "Conversation remains visible.".into(),
    ));
    app.update(AppEvent::ListSelectionPaneOpened(help_view()));

    let rendered = render(&app, 80, 24);

    assert!(rendered.contains("Conversation remains visible."));
    assert!(rendered.contains("Help"));
    assert!(rendered.contains("Commands"));
    assert!(rendered.contains("Keys"));
    assert!(rendered.contains("Space search"));
    assert!(rendered.contains("Search commands and shortcuts"));
    assert!(rendered.contains("Tab/Shift-Tab tabs"));
    assert!(!rendered.contains("enter send"));
    let rows = rendered.lines().collect::<Vec<_>>();
    let last_item_row = rows.iter().position(|row| row.contains("/model")).unwrap();
    let status_line_row = rows
        .iter()
        .position(|row| row.contains("Space search"))
        .unwrap();
    assert_eq!(status_line_row - last_item_row, 3);
}

#[test]
fn list_selection_pane_supports_keyboard_tab_switching_and_search() {
    let mut app = App::new();
    app.update(AppEvent::ListSelectionPaneOpened(help_view()));

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
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
fn selection_candidate_color_repaints_the_pane_and_welcome_frames() {
    let mut app = App::new();
    app.update(AppEvent::ListSelectionPaneOpened(PaneSpec::new(
        ListSelectionModel::new(
            "Theme",
            vec![ListSelectionGroup::new(
                "Themes",
                vec![
                    ListSelectionItem::new("First").with_selection_foreground(Color::Red),
                    ListSelectionItem::new("Second").with_selection_foreground(Color::Green),
                ],
            )],
        ),
        "Esc back",
    )));

    let first = render_buffer(&app, 80, 24);
    let interaction_y = super::layout(&app, Rect::new(0, 0, 80, 24))
        .input
        .panes
        .iter()
        .find(|entry| entry.kind == crate::components::chat_composer::ChatComposerPaneKind::Stacked)
        .unwrap()
        .area
        .y;
    assert_eq!(first[(2, 1)].fg, Color::Red);
    assert_eq!(first[(0, interaction_y)].fg, Color::Red);

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let second = render_buffer(&app, 80, 24);
    assert_eq!(second[(2, 1)].fg, Color::Green);
    assert_eq!(second[(0, interaction_y)].fg, Color::Green);
}

#[test]
fn error_detail_is_rendered_once_and_status_line_only_offers_recovery() {
    let mut app = App::new();
    app.update(AppEvent::FailureReported(
        "The configured model is unavailable.".into(),
    ));

    let rendered = render(&app, 80, 20);

    assert_eq!(
        rendered
            .matches("The configured model is unavailable.")
            .count(),
        1
    );
    assert!(rendered.contains("ask permissions on"));
    assert!(!rendered.contains("ready to retry"));
    assert!(!rendered.contains("esc esc rewind"));
    assert!(!rendered.contains("StableTurnError"));
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

    assert!(rows[0].contains("●  /theme zeta-code-light"));
    assert!(rows[1].contains("└─ Theme set to Zeta Code Light"));
    assert!(rows[2].trim().is_empty());
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
fn slash_popup_inherits_the_theme_surface_and_bolds_the_selected_command() {
    let mut app = App::new();
    app.insert_text("/");

    let buffer = render_buffer(&app, 80, 20);
    let selected = &buffer[(2, 10)];
    let unselected = &buffer[(2, 11)];
    let surface_background = buffer[(0, 0)].bg;

    assert_eq!(selected.fg, test_context().highlight());
    assert_eq!(selected.bg, surface_background);
    assert_eq!(selected.symbol(), "/");
    assert!(selected.modifier.contains(Modifier::BOLD));
    assert_eq!(unselected.fg, test_context().muted());
    assert_eq!(unselected.bg, surface_background);
    assert_eq!(unselected.symbol(), "/");
    assert!(!unselected.modifier.contains(Modifier::BOLD));
}

#[test]
fn slash_popup_hit_testing_maps_visible_rows_and_rejects_outside_clicks() {
    let mut app = App::new();
    app.insert_text("/");
    let terminal_area = Rect::new(0, 0, 80, 20);

    assert_eq!(input_overlay_index_at(&app, terminal_area, 2, 10), Some(0));
    assert_eq!(input_overlay_index_at(&app, terminal_area, 77, 15), Some(5));
    assert_eq!(input_overlay_index_at(&app, terminal_area, 1, 10), None);
    assert_eq!(input_overlay_index_at(&app, terminal_area, 2, 16), None);

    for _ in 0..7 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    assert_eq!(input_overlay_index_at(&app, terminal_area, 2, 10), Some(2));
    assert_eq!(input_overlay_index_at(&app, terminal_area, 2, 15), Some(7));
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
fn mention_popup_renders_paths_and_exposes_the_same_click_rows() {
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
    let Some(crate::components::suggest::SuggestView::Mention(popup)) = app.suggest() else {
        panic!("expected mention suggestions");
    };
    for (row, matched) in popup.matches.iter().take(2).enumerate() {
        for (column, character) in matched.label.chars().enumerate() {
            assert_eq!(
                buffer[(column as u16 + 2, row as u16 + 14)].symbol(),
                character.to_string()
            );
        }
    }
    let second = &popup.matches[1];
    let matched_index = second.indices[0];
    let unmatched_index = (0..second.label.chars().count())
        .find(|index| !second.indices.contains(index))
        .unwrap();
    assert!(
        buffer[(matched_index as u16 + 2, 15)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        !buffer[(unmatched_index as u16 + 2, 15)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(input_overlay_index_at(&app, terminal_area, 2, 14), Some(0));
    assert_eq!(input_overlay_index_at(&app, terminal_area, 2, 15), Some(1));
    assert_eq!(input_overlay_index_at(&app, terminal_area, 1, 15), None);
    let _ = fs::remove_dir_all(dir);
}

fn manager_session(
    id: &str,
    status: SessionManagerStatus,
    activity: Option<SessionManagerActivity>,
) -> Session {
    Session {
        session_id: SessionId::new(id).unwrap(),
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
        threads: Vec::new(),
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

fn help_view() -> PaneSpec<ListSelectionModel> {
    PaneSpec::new(
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
        .with_search(SearchBoxModel::new("Search commands and shortcuts")),
        "Space search  ·  Tab/Shift-Tab tabs  ·  ↑/↓ select  ·  Esc back",
    )
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
            app.suggest(),
            Some(crate::components::suggest::SuggestView::Mention(popup))
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
