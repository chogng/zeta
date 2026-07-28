use super::draw;
use super::history::estimated_wrapped_rows;
use super::mention_index_at;
use super::slash_command_index_at;
use super::theme::HIGHLIGHT;
use super::theme::MUTED;
use crate::app::App;
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
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[test]
fn wrapped_row_estimate_accounts_for_the_role_label() {
    assert_eq!(estimated_wrapped_rows(5, "hello", 10), 1);
    assert_eq!(estimated_wrapped_rows(5, "hello!", 10), 2);
}

#[test]
fn wrapped_row_estimate_uses_terminal_width_for_wide_characters() {
    assert_eq!(estimated_wrapped_rows(0, "你好", 4), 1);
    assert_eq!(estimated_wrapped_rows(0, "你好呀", 4), 2);
}

#[test]
fn wrapped_row_estimate_handles_an_unrenderable_width() {
    assert_eq!(estimated_wrapped_rows(5, "hello", 0), 0);
}

#[test]
fn empty_state_uses_lightweight_chrome_and_a_welcome_message() {
    let rendered = render(&App::new(), 80, 20);

    assert!(rendered.contains("Zeta  workspace assistant"));
    assert!(rendered.contains("Ask anything about your workspace."));
    assert!(rendered.contains("enter send  ·  ctrl-v image  ·  esc quit"));
    assert!(!rendered.contains("┌ Zeta"));
}

#[test]
fn error_detail_is_rendered_once_and_the_footer_only_offers_recovery() {
    let mut app = App::new();
    app.record_error("The configured model is unavailable.".into());

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
fn slash_popup_uses_gray_text_and_a_foreground_only_selection_highlight() {
    let mut app = App::new();
    app.insert_text("/");

    let buffer = render_buffer(&app, 80, 20);
    let selected = &buffer[(2, 10)];
    let unselected = &buffer[(2, 11)];

    assert_eq!(selected.fg, HIGHLIGHT);
    assert_eq!(selected.bg, Color::Reset);
    assert!(selected.modifier.contains(Modifier::BOLD));
    assert_eq!(unselected.fg, MUTED);
    assert_eq!(unselected.bg, Color::Reset);
}

#[test]
fn slash_popup_hit_testing_maps_visible_rows_and_rejects_outside_clicks() {
    let mut app = App::new();
    app.insert_text("/");
    let terminal_area = Rect::new(0, 0, 80, 20);

    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 10), Some(0));
    assert_eq!(slash_command_index_at(&app, terminal_area, 77, 15), Some(5));
    assert_eq!(slash_command_index_at(&app, terminal_area, 1, 10), None);
    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 16), None);

    for _ in 0..7 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 10), Some(2));
    assert_eq!(slash_command_index_at(&app, terminal_area, 2, 15), Some(7));
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
    wait_for_mention_results(&mut app);
    let terminal_area = Rect::new(0, 0, 80, 20);

    let buffer = render_buffer(&app, 80, 20);
    let popup = app.mention_popup().unwrap();
    for (row, matched) in popup.matches.iter().take(2).enumerate() {
        for (column, character) in matched.path.chars().enumerate() {
            assert_eq!(
                buffer[(column as u16 + 2, row as u16 + 14)].symbol(),
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
        buffer[(matched_index as u16 + 2, 15)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        !buffer[(unmatched_index as u16 + 2, 15)]
            .modifier
            .contains(Modifier::BOLD)
    );
    assert_eq!(mention_index_at(&app, terminal_area, 2, 14), Some(0));
    assert_eq!(mention_index_at(&app, terminal_area, 2, 15), Some(1));
    assert_eq!(mention_index_at(&app, terminal_area, 1, 15), None);
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

fn wait_for_mention_results(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.poll_background_events();
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
