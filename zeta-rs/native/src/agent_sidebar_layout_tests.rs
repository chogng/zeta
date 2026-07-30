use super::AgentSidebarLayout;
use zeta_ui::Rect;

#[test]
fn explorer_and_editor_are_sibling_panes_in_the_sidebar_grid() {
    let bounds = Rect::from_xywh(680.0, 32.0, 320.0, 668.0);
    let layout = AgentSidebarLayout::for_bounds(bounds);

    assert_eq!(
        layout.explorer(),
        Rect::from_xywh(680.0, 32.0, 320.0, 180.0)
    );
    assert_eq!(layout.editor(), Rect::from_xywh(680.0, 212.0, 320.0, 488.0));
    assert_eq!(layout.explorer().bottom(), layout.editor().origin.y);
    assert_eq!(layout.editor().bottom(), bounds.bottom());
}
