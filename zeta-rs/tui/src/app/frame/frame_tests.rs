use super::draw;
use super::mention_index_at;
use super::slash_command_index_at;
use crate::app::App;
use crate::app::AppEvent;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use crate::features::workspace_files::FileSearchManager;
use crate::ui::composer_chrome;
use crate::ui::highlight;
use crate::ui::muted;
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

#[test]
fn empty_frame_uses_lightweight_chrome_and_a_welcome_banner() {
    let rendered = render(&App::new(), 80, 20);

    assert!(!rendered.contains("workspace assistant"));
    assert!(rendered.contains(concat!("Zeta Code v", env!("CARGO_PKG_VERSION"))));
    assert!(rendered.contains("Welcome back!"));
    assert!(rendered.contains("Tips for getting started"));
    assert!(rendered.contains("Try asking"));
    assert!(
        rendered.contains(
            "policy  (shift + tab to cycle)  ·  enter send  ·  ctrl-v image  ·  esc quit"
        )
    );
}

#[test]
fn status_line_renders_workspace_context_above_the_composer() {
    let app = App::for_workspace(Path::new("/work/zeta"));
    let buffer = render_buffer(&app, 80, 20);
    let status_row = (0..80)
        .map(|x| buffer[(x, 15)].symbol())
        .collect::<String>();

    assert!(status_row.contains("/work/zeta"));
    assert_eq!(buffer[(77, 15)].fg, composer_chrome());
}

#[test]
fn composer_uses_light_gray_edge_to_edge_horizontal_rules_and_prompt() {
    let buffer = render_buffer(&App::new(), 80, 20);

    for y in [16, 18] {
        assert_eq!(buffer[(0, y)].symbol(), "─");
        assert_eq!(buffer[(0, y)].fg, composer_chrome());
        assert_eq!(buffer[(79, y)].symbol(), "─");
        assert_eq!(buffer[(79, y)].fg, composer_chrome());
    }
    assert_eq!(buffer[(0, 17)].symbol(), "❯");
    assert_eq!(buffer[(0, 17)].fg, composer_chrome());
    assert_eq!(buffer[(79, 17)].symbol(), " ");
}

#[test]
fn selection_view_replaces_the_composer_but_keeps_the_transcript_surface() {
    let mut app = App::new();
    app.update(AppEvent::ProductNotice(
        "Conversation remains visible.".into(),
    ));
    app.update(AppEvent::SelectionViewOpened(help_view()));

    let rendered = render(&app, 80, 24);

    assert!(rendered.contains("Conversation remains visible."));
    assert!(rendered.contains("Help"));
    assert!(rendered.contains("Commands"));
    assert!(rendered.contains("Keys"));
    assert!(rendered.contains("Space search"));
    assert!(!rendered.contains("Search commands and shortcuts"));
    assert!(rendered.contains("←/→ tabs"));
    assert!(!rendered.contains("enter send"));
}

#[test]
fn selection_view_supports_keyboard_tab_switching_and_search() {
    let mut app = App::new();
    app.update(AppEvent::SelectionViewOpened(help_view()));

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

    let rendered = render(&app, 80, 24);
    assert!(rendered.contains("Esc"));
    assert!(!rendered.contains("move selection"));

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.selection_view().is_some());
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.selection_view().is_none());
}

#[test]
fn selection_candidate_color_repaints_the_pane_and_welcome_frames() {
    let mut app = App::new();
    app.update(AppEvent::SelectionViewOpened(SelectionViewModel::new(
        "Theme",
        vec![SelectionTab::new(
            "Themes",
            vec![
                SelectionItem::new("First").with_selection_foreground(Color::Red),
                SelectionItem::new("Second").with_selection_foreground(Color::Green),
            ],
        )],
    )));

    let first = render_buffer(&app, 80, 24);
    let interaction_y = 24 - app.selection_view().unwrap().desired_height(80);
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
    assert!(rendered.contains("ready to retry  ·  esc quit"));
    assert!(!rendered.contains("StableTurnError"));
}

#[test]
fn bare_slash_renders_all_registered_commands() {
    let mut app = App::new();
    app.insert_text("/");

    let rendered = render(&app, 80, 20);

    assert!(rendered.contains("/status"));
    assert!(rendered.contains("/skills"));
    assert!(rendered.contains("/mcp"));
    assert!(rendered.contains("/resume"));
    assert!(rendered.contains("/clear"));
    assert!(rendered.contains("/config"));
    assert!(!rendered.contains("/login"));
    assert!(!rendered.contains("/plugins"));
}

#[test]
fn slash_popup_inherits_the_theme_surface_and_bolds_the_selected_command() {
    let mut app = App::new();
    app.insert_text("/");

    let buffer = render_buffer(&app, 80, 20);
    let selected = &buffer[(2, 9)];
    let unselected = &buffer[(2, 10)];
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

    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 9), Some(0));
    assert_eq!(slash_command_index_at(&app, terminal_area, 77, 14), Some(5));
    assert_eq!(slash_command_index_at(&app, terminal_area, 1, 9), None);
    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 15), None);

    for _ in 0..7 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 9), Some(2));
    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 14), Some(7));
}

#[test]
fn empty_slash_popup_has_no_clickable_command_rows() {
    let mut app = App::new();
    app.insert_text("/unknown");

    assert_eq!(
        slash_command_index_at(&app, Rect::new(0, 0, 80, 20), 2, 15),
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
    let popup = app.mention_popup().unwrap();
    for (row, matched) in popup.matches.iter().take(2).enumerate() {
        for (column, character) in matched.path.chars().enumerate() {
            assert_eq!(
                buffer[(column as u16 + 2, row as u16 + 13)].symbol(),
                character.to_string()
            );
        }
    }
    let second = &popup.matches[1];
    let matched_index = second.indices[0];
    let unmatched_index = (0..second.path.chars().count())
        .find(|index| !second.indices.contains(index))
        .unwrap();
    assert!(
        buffer[(matched_index as u16 + 2, 14)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        !buffer[(unmatched_index as u16 + 2, 14)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(mention_index_at(&app, terminal_area, 2, 13), Some(0));
    assert_eq!(mention_index_at(&app, terminal_area, 2, 14), Some(1));
    assert_eq!(mention_index_at(&app, terminal_area, 1, 14), None);
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

fn help_view() -> SelectionViewModel {
    SelectionViewModel::new(
        "Help",
        vec![
            SelectionTab::new(
                "Commands",
                vec![
                    SelectionItem::new("/status").with_description("show status"),
                    SelectionItem::new("/model").with_description("show model"),
                ],
            ),
            SelectionTab::new(
                "Keys",
                vec![
                    SelectionItem::new("↑ / ↓").with_description("move selection"),
                    SelectionItem::new("Esc").with_description("return to composer"),
                ],
            ),
        ],
    )
    .with_search_placeholder("Search commands and shortcuts")
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
        if app
            .mention_popup()
            .is_some_and(|popup| popup.matches.len() >= 2)
        {
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
