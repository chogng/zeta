use super::areas;
use super::desired_height;
use ratatui::layout::Rect;

#[test]
fn pane_layout_reserves_the_top_row_for_the_title_bar() {
    let pane = areas(Rect::new(3, 5, 80, 10));

    assert_eq!(pane.body, Rect::new(3, 6, 80, 9));
    assert_eq!(desired_height(7), 8);
}
