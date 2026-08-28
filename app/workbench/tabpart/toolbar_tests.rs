//! Body-mounted Workbench toolbar tests.

use super::{TOOLBAR_CONTENT_GAP, TOOLBAR_HEIGHT, TabContainerToolbar};
use crate::tabpart::identity::{
    ADD_SESSION, SESSION_SEARCH_INPUT, TAB_CONTAINER_ACTION_BAR, TAB_CONTAINER_TOGGLE,
    TAB_LAYOUT_MENU, TAB_LAYOUT_MENU_MOVE_TO_TITLEBAR, TAB_LAYOUT_MENU_TRIGGER,
};
use crate::tabpart::test_style;
use crate::{
    CaretVisibility, Color, Point, Rect, TabPart, TextInput, TextInputLayoutEngine, Titlebar,
    TitlebarInsets,
};
use zui::ui::UiIntent;
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
    assert_eq!(scene.icons().len(), 3);
    assert_eq!(scene.icons()[1].icon(), zeta_icons::icons::LAYOUT);
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
        frame.interaction().target_at(Point::new(170.0, 50.0)),
        Some(TAB_LAYOUT_MENU_TRIGGER)
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
        Some(UiIntent::Activate(ADD_SESSION))
    );
}

#[test]
fn layout_action_opens_the_move_to_titlebar_command() {
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
    let mut closed_frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    closed_frame.draw_component(&toolbar);
    let trigger = Point::new(170.0, 50.0);

    dispatch.pointer_moved(trigger, closed_frame.interaction());
    dispatch.press_primary(closed_frame.interaction());
    let _ = dispatch.release_primary(trigger, closed_frame.interaction());

    let toolbar = TabContainerToolbar::new(
        part_bounds,
        &TextInput::new(),
        CaretVisibility::Visible,
        test_style(),
        &mut text_layout,
        &dispatch,
    );
    let mut open_frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    let tab_part = TabPart::default();
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 32.0),
        test_style(),
        &tab_part,
        tab_part.active_tab_key(),
        true,
        None,
        TitlebarInsets::NONE,
        &dispatch,
    );
    open_frame.draw_component(&titlebar);
    open_frame.draw_component(&toolbar);
    let nodes = open_frame.interaction().accessibility_nodes(&dispatch);

    assert!(open_frame.interaction().node(TAB_LAYOUT_MENU).is_some());
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == TAB_LAYOUT_MENU_MOVE_TO_TITLEBAR)
            .map(|node| (node.role, node.label.as_str())),
        Some((AccessibilityRole::MenuItem, "Move tabs to titlebar"))
    );
    assert!(
        open_frame
            .scene()
            .text_blocks()
            .iter()
            .any(|text| text.text() == "Move tabs to titlebar")
    );

    let item = open_frame
        .interaction()
        .node(TAB_LAYOUT_MENU_MOVE_TO_TITLEBAR)
        .expect("layout menu command")
        .bounds();
    let item_center = Point::new(
        item.origin.x + item.size.width * 0.5,
        item.origin.y + item.size.height * 0.5,
    );
    dispatch.pointer_moved(item_center, open_frame.interaction());
    dispatch.press_primary(open_frame.interaction());
    assert_eq!(
        dispatch
            .release_primary(item_center, open_frame.interaction())
            .intent,
        Some(UiIntent::Activate(TAB_LAYOUT_MENU_MOVE_TO_TITLEBAR))
    );
    assert!(
        open_frame
            .interaction()
            .node(TAB_CONTAINER_TOGGLE)
            .is_some()
    );
}
