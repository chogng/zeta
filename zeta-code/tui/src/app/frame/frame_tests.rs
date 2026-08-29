use super::draw;
use super::input_overlay_index_at;
use crate::app::App;
use crate::app::AppEvent;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use crate::features::thread::TurnActivity;
use crate::features::workspace_files::FileSearchManager;
use crate::ui::accent;
use crate::ui::chat_input_chrome;
use crate::ui::danger;
use crate::ui::highlight;
use crate::ui::muted;
use crate::ui::warning;
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

#[test]
fn empty_frame_uses_lightweight_chrome_and_a_welcome_banner() {
    let rendered = render(&App::new(), 80, 20);

    assert!(!rendered.contains("workspace assistant"));
    assert!(rendered.contains(concat!("Zeta Code v", env!("CARGO_PKG_VERSION"))));
    assert!(rendered.contains("Welcome back!"));
    assert!(rendered.contains("Tips for getting started"));
    assert!(rendered.contains("Try asking"));
    assert!(!rendered.contains("enter send"));
    assert!(!rendered.contains("ctrl-v image"));
    let footer = rendered.lines().last().unwrap();
    assert_eq!(footer.trim_end(), "⏸ ask permissions on");
}

#[test]
fn footer_uses_a_distinct_symbol_for_each_approval_mode() {
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
            "⏸ ask permissions on",
            "⏩  auto review on",
            "▶ bypass permissions on",
        ]
    );
}

#[test]
fn footer_uses_a_distinct_color_for_each_approval_mode_symbol() {
    let mut app = App::new();
    let ask_permissions = render_buffer(&app, 80, 20);
    assert_eq!(ask_permissions[(0, 19)].fg, warning());
    assert_eq!(
        ask_permissions[("⏸".width() as u16, 19)].fg,
        chat_input_chrome()
    );

    app.set_next_approval_mode(zeta_protocol::ApprovalMode::AutoReview);
    let auto_review = render_buffer(&app, 80, 20);
    assert_eq!(auto_review[(0, 19)].fg, accent());
    assert_eq!(
        auto_review[("⏩".width() as u16, 19)].fg,
        chat_input_chrome()
    );

    app.set_next_approval_mode(zeta_protocol::ApprovalMode::BypassPermissions);
    let bypass_permissions = render_buffer(&app, 80, 20);
    assert_eq!(bypass_permissions[(0, 19)].fg, danger());
    assert_eq!(
        bypass_permissions[("▶".width() as u16, 19)].fg,
        chat_input_chrome()
    );
}

#[test]
fn footer_colors_current_and_next_modes_independently() {
    let mut app = App::new();
    app.set_current_approval_mode(Some(zeta_protocol::ApprovalMode::AskPermissions));
    app.set_next_approval_mode(zeta_protocol::ApprovalMode::AutoReview);

    let buffer = render_buffer(&app, 80, 20);
    let next_icon_column = "⏸ current: ask permissions on · ".width() as u16;
    assert_eq!(buffer[(0, 19)].fg, warning());
    assert_eq!(buffer[(next_icon_column, 19)].fg, accent());
}

#[test]
fn workspace_path_is_only_visible_in_the_empty_welcome_banner() {
    let mut app = App::for_workspace(Path::new("/work/zeta"));

    let empty = render(&app, 80, 20);
    assert!(empty.contains("/work/zeta"));
    assert!(!empty.lines().last().unwrap().contains("/work/zeta"));

    app.update(AppEvent::ProductNotice("Conversation started.".into()));
    assert!(!render(&app, 80, 20).contains("/work/zeta"));
}

#[test]
fn status_line_renders_model_in_the_footer() {
    let mut app = App::new();
    app.update(AppEvent::PreferredModelReceived(Some(ModelRefDto {
        provider: "anthropic".into(),
        model: "claude-sonnet".into(),
    })));

    let buffer = render_buffer(&app, 80, 20);
    let footer = (0..80)
        .map(|x| buffer[(x, 19)].symbol())
        .collect::<String>();

    assert!(footer.starts_with("⏸ ask permissions on"));
    assert!(footer.trim_end().ends_with("anthropic/claude-sonnet"));
    assert_eq!(buffer[(0, 19)].fg, warning());
}

#[test]
fn narrow_footer_keeps_the_first_configured_item() {
    let mut app = App::new();
    app.update(AppEvent::PreferredModelReceived(Some(ModelRefDto {
        provider: "anthropic".into(),
        model: "claude-sonnet".into(),
    })));

    let rendered = render(&app, 24, 20);
    let footer = rendered.lines().last().unwrap();

    assert_eq!(footer.trim_end(), "⏸ ask permissions on");
    assert!(!footer.contains("claude"));
}

#[test]
fn chat_input_uses_light_gray_edge_to_edge_horizontal_rules_and_prompt() {
    let buffer = render_buffer(&App::new(), 80, 20);

    for y in [16, 18] {
        assert_eq!(buffer[(0, y)].symbol(), "─");
        assert_eq!(buffer[(0, y)].fg, chat_input_chrome());
        assert_eq!(buffer[(79, y)].symbol(), "─");
        assert_eq!(buffer[(79, y)].fg, chat_input_chrome());
    }
    assert_eq!(buffer[(0, 17)].symbol(), "❯");
    assert_eq!(buffer[(0, 17)].fg, chat_input_chrome());
    assert_eq!(buffer[(79, 17)].symbol(), " ");
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
    assert_eq!(rows[19].trim_end(), "⏸ ask permissions on");
}

#[test]
fn working_footer_keeps_the_configured_context_without_runtime_text() {
    let mut app = App::new();
    app.update(AppEvent::TurnActivityChanged(TurnActivity::Working));

    let rendered = render(&app, 80, 20);
    let footer = rendered.lines().last().unwrap();

    assert_eq!(footer.trim_end(), "⏸ ask permissions on");
    assert!(!footer.contains("enter queue"));
    assert!(!footer.contains("ctrl-c interrupt"));
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
    assert!(rendered.contains("←/→ tabs"));
    assert!(!rendered.contains("enter send"));
    let rows = rendered.lines().collect::<Vec<_>>();
    let last_item_row = rows.iter().position(|row| row.contains("/model")).unwrap();
    let footer_row = rows
        .iter()
        .position(|row| row.contains("Space search"))
        .unwrap();
    assert_eq!(footer_row - last_item_row, 3);
}

#[test]
fn list_selection_pane_supports_keyboard_tab_switching_and_search() {
    let mut app = App::new();
    app.update(AppEvent::ListSelectionPaneOpened(help_view()));

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
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
        .height_entries
        .iter()
        .find(|entry| {
            entry.kind == crate::components::chat_input_area::ChatInputAreaHeightEntryKind::Pane
        })
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
fn error_detail_is_rendered_once_and_the_footer_only_offers_recovery() {
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

    assert_eq!(selected.fg, highlight());
    assert_eq!(selected.bg, surface_background);
    assert_eq!(selected.symbol(), "/");
    assert!(selected.modifier.contains(Modifier::BOLD));
    assert_eq!(unselected.fg, muted());
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
fn mention_popup_renders_workspace_paths_and_exposes_the_same_click_rows() {
    let workspace = std::env::temp_dir().join(format!(
        "zeta-tui-render-mention-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(workspace.join("docs")).unwrap();
    fs::create_dir_all(workspace.join("src")).unwrap();
    fs::write(workspace.join("docs/src-notes.md"), "notes").unwrap();
    fs::write(workspace.join("src/lib.rs"), "lib").unwrap();
    let mut app = App::for_workspace(&workspace);
    app.insert_text("@src");
    wait_for_mention_results(&mut app, &workspace);
    let terminal_area = Rect::new(0, 0, 80, 20);

    let buffer = render_buffer(&app, 80, 20);
    let Some(crate::components::chat_input::SuggestView::Mention(popup)) = app.suggest() else {
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
    let _ = fs::remove_dir_all(workspace);
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
        "Space search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Esc back",
    )
}

fn wait_for_mention_results(app: &mut App, workspace: &Path) {
    let mut file_search = FileSearchManager::new(workspace.to_path_buf());
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
            Some(crate::components::chat_input::SuggestView::Mention(popup))
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
