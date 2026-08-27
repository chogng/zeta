//! Body-mounted Workbench toolbar tests.

use super::{TOOLBAR_CONTENT_GAP, TOOLBAR_HEIGHT, TabContainerToolbar};
use crate::workbench::identity::{ADD_SESSION, SESSION_SEARCH_INPUT, TAB_CONTAINER_ACTION_BAR};
use crate::workbench::test_style;
use crate::{CaretVisibility, Color, Point, Rect, TextInput, TextInputLayoutEngine};
use zui::ui::{AccessibilityRole, InteractionFrame, UiDispatch, UiFrame};

#[test]
fn toolbar_fills_the_container_row_with_search_and_add_action() {
    let part_bounds = Rect::from_xywh(0.0, 32.0, 220.0, 668.0);
    let mut dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let toolbar = TabContainerToolbar::new(
        part_bounds,
        &TextInput::new(),
        CaretVisibility::Visible,
        test_style(),
        &mut text_layout,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&toolbar);
    let scene = frame.scene();

    assert_eq!(toolbar.bounds.size.width, 220.0);
    assert_eq!(toolbar.bounds.size.height, TOOLBAR_HEIGHT);
    assert_eq!(
        TabContainerToolbar::content_bounds(part_bounds).origin.y - toolbar.bounds.bottom(),
        TOOLBAR_CONTENT_GAP
    );
    assert_eq!(scene.text_blocks()[0].text(), "Search sessions...");
    let target = scene
        .inspection()
        .target_at(Point::new(20.0, 50.0))
        .expect("search input should be inspectable");
    assert_eq!(
        scene
            .inspection()
            .ancestry(target.id())
            .iter()
            .map(|node| node.name())
            .collect::<Vec<_>>(),
        vec!["TabContainerToolbar", "SearchBox", "InputBox"]
    );
    assert_eq!(scene.icons().len(), 2);
    assert!(
        scene
            .icons()
            .iter()
            .all(|icon| icon.bounds().size.width == 18.0 && icon.bounds().size.height == 18.0)
    );
    assert_eq!(
        frame.interaction().target_at(Point::new(20.0, 50.0)),
        Some(SESSION_SEARCH_INPUT)
    );
    assert_eq!(
        frame.interaction().target_at(Point::new(196.0, 50.0)),
        Some(ADD_SESSION)
    );
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
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
            .find(|node| node.id == TAB_CONTAINER_ACTION_BAR)
            .unwrap()
            .role,
        AccessibilityRole::Toolbar
    );

    let add_session_point = Point::new(196.0, 50.0);
    dispatch.pointer_moved(add_session_point, frame.interaction());
    dispatch.press_primary(frame.interaction());
    assert_eq!(
        dispatch
            .release_primary(add_session_point, frame.interaction())
            .intent,
        Some(zui::ui::UiIntent::Activate(ADD_SESSION))
    );
}
