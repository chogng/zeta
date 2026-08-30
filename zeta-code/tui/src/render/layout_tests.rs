use super::bottom_anchored_area;
use ratatui::layout::Rect;

#[test]
fn popup_area_is_clamped_and_anchored_to_its_parent_bottom() {
    let parent = Rect::new(5, 7, 80, 6);

    assert_eq!(bottom_anchored_area(parent, 4), Rect::new(5, 9, 80, 4));
    assert_eq!(bottom_anchored_area(parent, 99), parent);
}
