//! Cross-surface interaction, focus, activation, and semantics regression tests.

use zeta_ui::{Point, Rect};

use super::{
    AccessibilityRole, CursorFeedback, DispatchInvalidation, ElementId, FocusBehavior,
    FocusDirection, InteractionFrame, NavigationAxis, NavigationGroupId, NodeAction, UiDispatch,
    UiIntent, UiNode,
};

const ROOT: ElementId = ElementId::scoped(1, 1);
const INPUT: ElementId = ElementId::scoped(1, 2);
const TOOLBAR: ElementId = ElementId::scoped(1, 3);
const FIRST: ElementId = ElementId::scoped(1, 4);
const SECOND: ElementId = ElementId::scoped(1, 5);

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
