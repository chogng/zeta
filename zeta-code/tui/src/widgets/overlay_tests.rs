use super::DetailOverlay;
use super::draw;
use crate::render::test_context;
use crate::widgets::detail_list::DetailList;
use crate::widgets::detail_list::DetailListRow;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use insta::assert_snapshot;
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
    let available = ratatui::layout::Rect::new(0, 0, 20, 4);

    overlay.handle_key(
        KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL),
        available,
    );
    let end = overlay.scroll;
    assert!(end > 0);
    overlay.handle_key(
        KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
        available,
    );
    assert_eq!(overlay.scroll, end);
    overlay.handle_key(
        KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL),
        available,
    );
    assert_eq!(overlay.scroll, 0);
}

#[test]
fn wrapped_content_uses_the_same_width_for_height_and_scroll() {
    let detail = DetailList::new(
        "Output",
        vec![DetailListRow::new(
            "stdout",
            "a result that wraps across several narrow terminal rows",
        )],
    );
    let mut overlay = DetailOverlay::new(detail);
    let available = ratatui::layout::Rect::new(0, 0, 18, 5);

    overlay.handle_key(
        KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL),
        available,
    );

    assert!(overlay.scroll > 0);
}

#[test]
fn overlay_fills_each_wide_row_and_aligns_content_with_the_page() {
    let detail = DetailList::new("Output", vec![DetailListRow::new("stdout", "done")]);
    let overlay = DetailOverlay::new(detail);
    let mut terminal = Terminal::new(TestBackend::new(120, 8)).unwrap();

    terminal
        .draw(|frame| {
            frame.render_widget(Paragraph::new("underlying text ".repeat(8)), frame.area());
            draw(frame, frame.area(), &overlay, test_context());
        })
        .unwrap();

    assert_eq!(terminal.backend().buffer()[(0, 2)].symbol(), " ");
    assert_eq!(terminal.backend().buffer()[(119, 2)].symbol(), " ");
    assert_eq!(
        terminal.backend().buffer()[(0, 2)].bg,
        Color::Rgb(37, 37, 38)
    );
    assert_eq!(
        terminal.backend().buffer()[(119, 2)].bg,
        Color::Rgb(37, 37, 38)
    );
    assert_eq!(terminal.backend().buffer()[(2, 2)].symbol(), "O");
    assert_snapshot!("wide_detail_overlay_uses_full_rows", terminal.backend());
}
