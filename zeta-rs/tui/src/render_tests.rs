use super::draw;
use super::estimated_wrapped_rows;
use crate::app::App;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

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
    assert!(rendered.contains("enter send  ·  esc quit"));
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

fn render(app: &App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| draw(frame, app)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
