//! Cross-surface interaction, focus, activation, and semantics regression tests.

use zui::{Point, Rect};

use super::{
    AccessibilityExpansion, AccessibilityRole, CursorFeedback, DispatchInvalidation, ElementId,
    FocusBehavior, FocusDirection, InteractionFrame, NavigationAxis, NavigationGroupId, NodeAction,
    UiDispatch, UiIntent, UiNode,
};

const ROOT: ElementId = ElementId::scoped(1, 1);
const INPUT: ElementId = ElementId::scoped(1, 2);
const TOOLBAR: ElementId = ElementId::scoped(1, 3);
const FIRST: ElementId = ElementId::scoped(1, 4);
const SECOND: ElementId = ElementId::scoped(1, 5);
const MENU: ElementId = ElementId::scoped(1, 6);
const MENU_ITEM: ElementId = ElementId::scoped(1, 7);

fn frame() -> InteractionFrame {
    let mut frame = InteractionFrame::default();
    frame.register(UiNode::new(
        ROOT,
        Rect::from_xywh(0.0, 0.0, 300.0, 200.0),
        AccessibilityRole::Window,
        "zeterm",
    ));
    frame.register(
        UiNode::new(
            INPUT,
            Rect::from_xywh(20.0, 20.0, 260.0, 40.0),
            AccessibilityRole::TextInput,
            "Command input",
        )
        .with_parent(ROOT)
        .with_cursor(CursorFeedback::Text)
        .with_focus(FocusBehavior::TabStop),
    );
    frame.register(
        UiNode::new(
            TOOLBAR,
            Rect::from_xywh(20.0, 80.0, 260.0, 30.0),
            AccessibilityRole::Toolbar,
            "Input context",
        )
        .with_parent(ROOT),
    );
    let group = NavigationGroupId::new(TOOLBAR);
    for (id, x, label) in [(FIRST, 20.0, "Local"), (SECOND, 90.0, "Working directory")] {
        frame.register(
            UiNode::new(
                id,
                Rect::from_xywh(x, 80.0, 60.0, 24.0),
                AccessibilityRole::Button,
                label,
            )
            .with_parent(TOOLBAR)
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate)
            .with_navigation(group, NavigationAxis::Horizontal),
        );
    }
    frame
}

fn frame_with_modal_menu() -> InteractionFrame {
    let mut frame = frame();
    frame.register(
        UiNode::new(
            MENU,
            Rect::from_xywh(100.0, 40.0, 120.0, 80.0),
            AccessibilityRole::Menu,
            "Actions",
        )
        .with_parent(ROOT),
    );
    frame.register(
        UiNode::new(
            MENU_ITEM,
            Rect::from_xywh(102.0, 42.0, 116.0, 30.0),
            AccessibilityRole::MenuItem,
            "Close",
        )
        .with_parent(MENU)
        .with_cursor(CursorFeedback::Pointer)
        .with_focus(FocusBehavior::TabStop)
        .with_action(NodeAction::Activate),
    );
    frame.set_modal_root(MENU);
    frame
}

#[test]
fn hit_testing_prefers_the_last_painted_child_and_projects_ancestry_hover() {
    let frame = frame();
    let mut dispatch = UiDispatch::default();

    assert_eq!(
        dispatch
            .pointer_moved(Point::new(25.0, 85.0), &frame)
            .invalidation,
        DispatchInvalidation::Paint
    );
    assert!(dispatch.is_hovered(ROOT));
    assert!(dispatch.is_hovered(TOOLBAR));
    assert!(dispatch.is_hovered(FIRST));
    assert_eq!(dispatch.pointer_feedback(&frame), CursorFeedback::Pointer);
}

#[test]
fn host_resolved_hover_projects_a_stable_element_without_pointer_movement() {
    let frame = frame();
    let mut dispatch = UiDispatch::default();
    dispatch.pointer_moved(Point::new(25.0, 85.0), &frame);

    assert_eq!(
        dispatch.hover_element(SECOND, &frame).invalidation,
        DispatchInvalidation::Paint
    );
    assert!(dispatch.is_hovered(ROOT));
    assert!(dispatch.is_hovered(TOOLBAR));
    assert!(dispatch.is_hovered(SECOND));
    assert!(!dispatch.is_hovered(FIRST));
}

#[test]
fn interaction_checkpoint_restores_nodes_and_modal_scope() {
    let mut frame = frame();
    let checkpoint = frame.checkpoint();
    frame.register(
        UiNode::new(
            MENU,
            Rect::from_xywh(100.0, 40.0, 120.0, 80.0),
            AccessibilityRole::Menu,
            "Actions",
        )
        .with_parent(ROOT),
    );
    frame.set_modal_root(MENU);
    assert_eq!(frame.target_at(Point::new(25.0, 85.0)), None);

    frame.restore(checkpoint);

    assert!(frame.node(MENU).is_none());
    assert_eq!(frame.target_at(Point::new(25.0, 85.0)), Some(FIRST));
}

#[test]
fn pointer_capture_only_activates_when_release_returns_to_the_pressed_button() {
    let frame = frame();
    let mut dispatch = UiDispatch::default();
    dispatch.pointer_moved(Point::new(25.0, 85.0), &frame);

    dispatch.press_primary(&frame);
    dispatch.pointer_moved(Point::new(200.0, 150.0), &frame);
    assert_eq!(
        dispatch
            .release_primary(Point::new(200.0, 150.0), &frame)
            .intent,
        None
    );

    dispatch.pointer_moved(Point::new(25.0, 85.0), &frame);
    dispatch.press_primary(&frame);
    assert_eq!(
        dispatch
            .release_primary(Point::new(25.0, 85.0), &frame)
            .intent,
        Some(UiIntent::Activate(FIRST))
    );
}

#[test]
fn tab_order_and_horizontal_navigation_share_the_same_focus_owner() {
    let frame = frame();
    let mut dispatch = UiDispatch::default();
    dispatch.reconcile_focus(&frame, INPUT);
    assert!(dispatch.is_focused(INPUT));

    dispatch.focus_in_order(&frame, FocusDirection::Next);
    assert!(dispatch.is_focused(FIRST));
    dispatch.focus_within_group(&frame, FocusDirection::Next, NavigationAxis::Horizontal);
    assert!(dispatch.is_focused(SECOND));
    dispatch.focus_in_order(&frame, FocusDirection::Previous);
    assert!(dispatch.is_focused(FIRST));
}

#[test]
fn keyboard_activation_and_accessibility_use_the_focused_node_identity() {
    let frame = frame();
    let mut dispatch = UiDispatch::default();
    dispatch.reconcile_focus(&frame, FIRST);

    assert_eq!(
        dispatch.activate_focused(&frame).intent,
        Some(UiIntent::Activate(FIRST))
    );
    let nodes = frame.accessibility_nodes(&dispatch);
    let button = nodes.iter().find(|node| node.id == FIRST).unwrap();
    assert_eq!(button.parent, Some(TOOLBAR));
    assert_eq!(button.role, AccessibilityRole::Button);
    assert_eq!(button.label, "Local");
    assert_eq!(button.value, None);
    assert_eq!(
        button.selection,
        super::AccessibilitySelection::NotApplicable
    );
    assert!(button.focusable);
    assert!(button.focused);
    assert_eq!(button.bounds, Rect::from_xywh(20.0, 80.0, 60.0, 24.0));
}

#[test]
fn tree_item_accessibility_retains_level_and_expansion() {
    let tree = ElementId::scoped(2, 1);
    let item = ElementId::scoped(2, 2);
    let mut frame = InteractionFrame::default();
    frame.register(UiNode::new(
        tree,
        Rect::from_xywh(0.0, 0.0, 200.0, 100.0),
        AccessibilityRole::Tree,
        "Files",
    ));
    frame.register(
        UiNode::new(
            item,
            Rect::from_xywh(0.0, 0.0, 200.0, 24.0),
            AccessibilityRole::TreeItem,
            "src",
        )
        .with_parent(tree)
        .with_level(2)
        .with_expansion(AccessibilityExpansion::Expanded),
    );

    let nodes = frame.accessibility_nodes(&UiDispatch::default());
    let item = nodes.iter().find(|node| node.id == item).unwrap();
    assert_eq!(item.level, Some(2));
    assert_eq!(item.expansion, AccessibilityExpansion::Expanded);
}

#[test]
fn window_blur_clears_transient_pointer_state_but_retains_focus_identity() {
    let frame = frame();
    let mut dispatch = UiDispatch::default();
    dispatch.reconcile_focus(&frame, FIRST);
    dispatch.pointer_moved(Point::new(25.0, 85.0), &frame);
    dispatch.press_primary(&frame);

    dispatch.window_blurred();

    assert!(!dispatch.is_hovered(FIRST));
    assert!(!dispatch.is_pressed(FIRST));
    assert!(!dispatch.is_focused(FIRST));
    assert_eq!(dispatch.focused(), Some(FIRST));
    dispatch.window_focused();
    assert!(dispatch.is_focused(FIRST));
}

#[test]
fn modal_root_makes_background_pointer_targets_inert() {
    let frame = frame_with_modal_menu();
    let mut dispatch = UiDispatch::default();

    assert_eq!(frame.target_at(Point::new(25.0, 25.0)), None);
    assert_eq!(frame.target_at(Point::new(110.0, 50.0)), Some(MENU_ITEM));
    dispatch.pointer_moved(Point::new(25.0, 25.0), &frame);
    assert!(!dispatch.is_hovered(ROOT));
    assert!(!dispatch.is_hovered(INPUT));
    dispatch.pointer_moved(Point::new(110.0, 50.0), &frame);
    assert!(dispatch.is_hovered(MENU));
    assert!(dispatch.is_hovered(MENU_ITEM));
    assert!(!dispatch.is_hovered(ROOT));
}

#[test]
fn modal_root_traps_focus_and_activation_in_its_subtree() {
    let base_frame = frame();
    let modal_frame = frame_with_modal_menu();
    let mut dispatch = UiDispatch::default();
    dispatch.reconcile_focus(&base_frame, INPUT);

    dispatch.reconcile_focus(&modal_frame, INPUT);

    assert!(dispatch.is_focused(MENU_ITEM));
    assert_eq!(
        modal_frame.focus_order().collect::<Vec<_>>(),
        vec![MENU_ITEM]
    );
    dispatch.focus_element(&modal_frame, INPUT);
    assert!(dispatch.is_focused(MENU_ITEM));
    assert_eq!(
        dispatch.activate_focused(&modal_frame).intent,
        Some(UiIntent::Activate(MENU_ITEM))
    );
    let nodes = modal_frame.accessibility_nodes(&dispatch);
    assert!(
        !nodes
            .iter()
            .find(|node| node.id == INPUT)
            .unwrap()
            .focusable
    );
    assert!(
        nodes
            .iter()
            .find(|node| node.id == MENU_ITEM)
            .unwrap()
            .focusable
    );
}
