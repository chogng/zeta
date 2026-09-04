use super::KeyHints;
use super::draw;
use super::draw_right;
use crate::render::test_context;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn key_hints_format_actions_and_notes_in_order() {
    let hints = KeyHints::new()
        .with_action("Enter", "apply")
        .with_note("current: dark")
        .with_action("Esc", "close");

    assert_eq!(
        hints.text(),
        "Enter to apply  ·  current: dark  ·  Esc to close"
    );
}

#[test]
fn key_hint_uses_two_character_horizontal_insets() {
    let backend = TestBackend::new(30, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| draw(frame, frame.area(), "Enter to apply", test_context()))
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), " ");
    assert_eq!(buffer[(1, 0)].symbol(), " ");
    assert_eq!(buffer[(2, 0)].symbol(), "E");
}

#[test]
fn right_aligned_key_hint_keeps_the_two_character_right_inset() {
    let backend = TestBackend::new(30, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| draw_right(frame, frame.area(), "← for agents", test_context()))
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(16, 0)].symbol(), "←");
    assert_eq!(buffer[(27, 0)].symbol(), "s");
    assert_eq!(buffer[(28, 0)].symbol(), " ");
    assert_eq!(buffer[(29, 0)].symbol(), " ");
}
