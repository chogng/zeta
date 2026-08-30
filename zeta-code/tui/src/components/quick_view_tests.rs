use super::QuickViewState;
use super::draw;
use crate::components::detail_list::DetailList;
use crate::components::detail_list::DetailListRow;
use crate::components::pane::PaneSpec;
use crate::render::test_context;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

#[test]
fn detail_scroll_is_bounded_and_supports_first_and_last_shortcuts() {
    let detail = DetailList::new(
        "Output",
        vec![DetailListRow::new("stdout", "one\ntwo\nthree")],
    );
    let mut quick_view = QuickViewState::new(PaneSpec::new(detail, "Esc close"));

    quick_view.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL));
    assert_eq!(quick_view.scroll, quick_view.max_scroll());
    quick_view.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(quick_view.scroll, quick_view.max_scroll());
    quick_view.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL));
    assert_eq!(quick_view.scroll, 0);
}

#[test]
fn overlay_fills_its_surface_with_the_quick_view_background() {
    let detail = DetailList::new("Output", vec![DetailListRow::new("stdout", "done")]);
    let quick_view = QuickViewState::new(PaneSpec::new(detail, "Esc close"));
    let mut terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();

    terminal
        .draw(|frame| draw(frame, frame.area(), &quick_view, test_context()))
        .unwrap();

    assert_eq!(
        terminal.backend().buffer()[(0, 2)].bg,
        Color::Rgb(37, 37, 38)
    );
}
