use super::AgentSidebarLayout;
use zeta_ui::Rect;

#[test]
fn sidebar_resolves_a_full_width_toolbar_and_single_content_pane() {
    let bounds = Rect::from_xywh(680.0, 32.0, 320.0, 668.0);
    let layout = AgentSidebarLayout::for_bounds(bounds);

    assert_eq!(layout.toolbar(), Rect::from_xywh(680.0, 32.0, 320.0, 36.0));
    assert_eq!(layout.content(), Rect::from_xywh(680.0, 68.0, 320.0, 632.0));
}
