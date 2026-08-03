use super::{SCM_TOOLBAR_HEIGHT, ScmLayout};
use zeta_ui::Rect;

#[test]
fn scm_layout_resolves_toolbar_and_content() {
    let layout = ScmLayout::for_bounds(Rect::from_xywh(10.0, 20.0, 320.0, 200.0));

    assert_eq!(layout.toolbar().size.height, SCM_TOOLBAR_HEIGHT);
    assert_eq!(layout.content().origin.y, 20.0 + SCM_TOOLBAR_HEIGHT);
    assert_eq!(layout.content().size.height, 200.0 - SCM_TOOLBAR_HEIGHT);
}
