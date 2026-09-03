use super::DetailOverlay;
use super::draw;
use crate::components::detail_list::DetailList;
use crate::components::detail_list::DetailListRow;
use crate::render::test_context;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::widgets::Paragraph;

#[test]
fn detail_scroll_is_bounded_and_supports_first_and_last_shortcuts() {
    let detail = DetailList::new(
        "Output",
        vec![DetailListRow::new("stdout", "one\ntwo\nthree")],
    );
    let mut overlay = DetailOverlay::new(detail);

    overlay.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL));
    assert_eq!(overlay.scroll, overlay.max_scroll());
    overlay.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(overlay.scroll, overlay.max_scroll());
    overlay.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL));
    assert_eq!(overlay.scroll, 0);
}

#[test]
fn overlay_fills_its_surface_with_the_overlay_background() {
    let detail = DetailList::new("Output", vec![DetailListRow::new("stdout", "done")]);
    let overlay = DetailOverlay::new(detail);
    let mut terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();

    terminal
        .draw(|frame| {
            frame.render_widget(Paragraph::new("underlying text"), frame.area());
            draw(frame, frame.area(), &overlay, test_context());
        })
        .unwrap();

    assert_eq!(terminal.backend().buffer()[(0, 2)].symbol(), " ");
    assert_eq!(
        terminal.backend().buffer()[(0, 2)].bg,
        Color::Rgb(37, 37, 38)
    );
}
