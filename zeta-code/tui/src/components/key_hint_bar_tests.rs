use super::draw;
use crate::render::test_context;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn key_hint_bar_uses_two_character_horizontal_insets() {
    let backend = TestBackend::new(30, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| draw(frame, frame.area(), "↑/↓ select", test_context()))
        .unwrap();

    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), " ");
    assert_eq!(buffer[(1, 0)].symbol(), " ");
    assert_eq!(buffer[(2, 0)].symbol(), "↑");
}
