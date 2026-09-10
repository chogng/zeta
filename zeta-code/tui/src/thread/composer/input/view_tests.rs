use super::ChatInputChrome;
use super::ChatInputCursor;
use super::ChatInputFocus;
use crate::render::test_context;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

fn border(focus: ChatInputFocus) -> ratatui::style::Color {
    let area = Rect::new(0, 0, 40, 4);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    terminal
        .draw(|frame| {
            super::draw(
                frame,
                area,
                "",
                0,
                0,
                "> ",
                ChatInputCursor::Hidden,
                focus,
                ChatInputChrome::Box,
                test_context(),
            )
        })
        .unwrap();
    let x = super::content_area(area).x;
    terminal.backend().buffer()[(x, area.y)].fg
}

#[test]
fn input_border_distinguishes_focus_from_rest() {
    assert_eq!(
        border(ChatInputFocus::Focused),
        test_context().chat_input_chrome()
    );
    assert_eq!(border(ChatInputFocus::Blurred), test_context().border());
}
