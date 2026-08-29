use super::areas;
use ratatui::layout::Rect;

#[test]
fn divides_chat_history_input_area_and_footer() {
    let areas = areas(Rect::new(0, 0, 80, 24), 9);

    assert_eq!(areas.chat_history, Rect::new(0, 0, 80, 14));
    assert_eq!(areas.chat_input_area, Rect::new(0, 14, 80, 9));
    assert_eq!(areas.footer, Rect::new(0, 23, 80, 1));
}

#[test]
fn preserves_history_and_footer_when_input_area_is_oversized() {
    let areas = areas(Rect::new(5, 7, 80, 24), 99);

    assert_eq!(areas.chat_history, Rect::new(5, 7, 80, 4));
    assert_eq!(areas.chat_input_area, Rect::new(5, 11, 80, 19));
    assert_eq!(areas.footer, Rect::new(5, 30, 80, 1));
}
