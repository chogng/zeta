use super::body_area;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionModel;
use crate::components::list_selection::ListSelectionState;
use crate::components::region::RegionView;
use crate::components::region::view_desired_height;
use ratatui::layout::Rect;

#[test]
fn region_reserves_one_row_for_its_title() {
    let area = Rect::new(3, 5, 80, 10);
    assert_eq!(body_area(area), Rect::new(3, 6, 80, 9));

    let state = ListSelectionState::new(ListSelectionModel::new(
        "Help",
        vec![ListSelectionGroup::new(
            "Commands",
            vec![ListSelectionItem::new("/status")],
        )],
    ));
    assert_eq!(
        view_desired_height(RegionView::ListSelection(&state), 80),
        state.desired_height(80) + 1
    );
}
