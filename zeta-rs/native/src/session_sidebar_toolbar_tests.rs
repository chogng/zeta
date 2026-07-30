use super::{SessionSidebarToolbar, TOOLBAR_CONTENT_GAP, TOOLBAR_HEIGHT};
use crate::shell_interaction::{ADD_SESSION, SESSION_SEARCH_INPUT, SESSION_SIDEBAR_ACTION_BAR};
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{
    CaretVisibility, Color, Component, Point, Rect, TextInput, TextInputLayoutEngine, UiScene,
};
use zeta_ui_dispatch::{AccessibilityRole, InteractionFrame, UiDispatch};

#[test]
fn toolbar_fills_the_sidebar_row_with_search_and_add_action() {
    let sidebar_bounds = Rect::from_xywh(0.0, 32.0, 220.0, 668.0);
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let toolbar = SessionSidebarToolbar::new(
        sidebar_bounds,
        &TextInput::new(),
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);
    let mut frame = InteractionFrame::default();

    toolbar.paint(&mut scene);
    toolbar.register_interactions(&mut frame);

    assert_eq!(toolbar.bounds.size.width, 220.0);
    assert_eq!(toolbar.bounds.size.height, TOOLBAR_HEIGHT);
    assert_eq!(
        SessionSidebarToolbar::content_bounds(sidebar_bounds)
            .origin
            .y
            - toolbar.bounds.bottom(),
        TOOLBAR_CONTENT_GAP
    );
    assert_eq!(scene.text_blocks()[0].text(), "Search sessions...");
    assert_eq!(scene.icons().len(), 2);
    assert!(
        scene
            .icons()
            .iter()
            .all(|icon| icon.bounds().size.width == 18.0 && icon.bounds().size.height == 18.0)
    );
    assert_eq!(
        frame.target_at(Point::new(20.0, 50.0)),
        Some(SESSION_SEARCH_INPUT)
    );
    assert_eq!(frame.target_at(Point::new(196.0, 50.0)), Some(ADD_SESSION));
    let nodes = frame.accessibility_nodes(&dispatch);
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == ADD_SESSION)
            .unwrap()
            .role,
        AccessibilityRole::Button
    );
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == SESSION_SIDEBAR_ACTION_BAR)
            .unwrap()
            .role,
        AccessibilityRole::Toolbar
    );
}
