use super::areas;
use super::desired_height;
use ratatui::layout::Rect;

#[test]
fn pane_layout_reserves_two_rows_between_body_and_key_hint_bar() {
    let pane = areas(Rect::new(3, 5, 80, 10));

    assert_eq!(pane.body, Rect::new(3, 5, 80, 7));
    assert_eq!(pane.key_hint_bar, Rect::new(3, 14, 80, 1));
    assert_eq!(pane.key_hint_bar.y - (pane.body.y + pane.body.height), 2);
    assert_eq!(desired_height(7), 10);
}
