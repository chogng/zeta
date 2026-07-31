use super::InputContextToolbar;
use crate::agent_composer::ComposerMode;
use crate::shell_interaction::ContextAction;
use crate::shell_style::SHELL_PALETTE;
use crate::workspace_context::WorkspaceContext;
use zeta_ui::{Component, Point, Rect, TextInputLayoutEngine, UiScene};
use zeta_ui_dispatch::{InteractionFrame, UiDispatch};

#[test]
fn toolbar_projects_mode_and_four_real_context_values_as_action_buttons() {
    let context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(7));
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let toolbar = InputContextToolbar::new(
        Rect::from_xywh(24.0, 600.0, 952.0, 24.0),
        &context,
        ComposerMode::Agent,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    );
    let mut scene = UiScene::new(SHELL_PALETTE.background);

    toolbar.paint(&mut scene);

    assert_eq!(scene.icons().len(), 5);
    assert_eq!(
        scene
            .text_blocks()
            .iter()
            .map(|block| block.text())
            .collect::<Vec<_>>(),
        [
            "Agent",
            "Local",
            "~/Desktop/zeta",
            "main",
            "Changes 7 • +7 -0"
        ]
    );
    assert_eq!(scene.rects().len(), 5);
    assert!(toolbar.item_bounds(0).unwrap().right() < toolbar.item_bounds(1).unwrap().origin.x);
    assert_eq!(toolbar.hit_test(Point::new(40.0, 612.0)), Some(0));
    assert!(
        toolbar.item_bounds(2).unwrap().size.width > toolbar.item_bounds(3).unwrap().size.width
    );
}

#[test]
fn toolbar_scales_all_items_into_a_narrow_input_surface() {
    let context = WorkspaceContext::fixture("/tmp/project", None, None);
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let toolbar = InputContextToolbar::new(
        Rect::from_xywh(24.0, 200.0, 192.0, 24.0),
        &context,
        ComposerMode::Agent,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    );

    assert_eq!(toolbar.item_bounds(0).unwrap().origin.x, 24.0);
    assert!(toolbar.item_bounds(4).unwrap().right() <= 216.0);
    assert!(toolbar.item_bounds(5).is_none());
}

#[test]
fn toolbar_registers_the_same_button_bounds_used_for_painting() {
    let context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(7));
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let toolbar = InputContextToolbar::new(
        Rect::from_xywh(24.0, 600.0, 952.0, 24.0),
        &context,
        ComposerMode::Agent,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    );
    let mut frame = InteractionFrame::default();

    toolbar.register_interactions(&mut frame);

    let location = toolbar.item_bounds(1).unwrap();
    assert_eq!(
        frame.target_at(Point::new(location.origin.x + 1.0, location.origin.y + 1.0)),
        Some(ContextAction::Location.element_id())
    );
}

#[test]
fn toolbar_projects_host_hover_state_back_into_the_hit_button() {
    let context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(7));
    let mut text_layout = TextInputLayoutEngine::new();
    let mut dispatch = UiDispatch::default();
    let resting = InputContextToolbar::new(
        Rect::from_xywh(24.0, 600.0, 952.0, 24.0),
        &context,
        ComposerMode::Agent,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    );
    let mut frame = InteractionFrame::default();
    resting.register_interactions(&mut frame);
    let first = resting.item_bounds(0).unwrap();
    dispatch.pointer_moved(
        Point::new(first.origin.x + 1.0, first.origin.y + 1.0),
        &frame,
    );
    let hovered = InputContextToolbar::new(
        Rect::from_xywh(24.0, 600.0, 952.0, 24.0),
        &context,
        ComposerMode::Agent,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    );
    let mut scene = UiScene::new(SHELL_PALETTE.background);

    hovered.paint(&mut scene);

    assert_eq!(scene.rects()[0].fill(), SHELL_PALETTE.surface_hovered);
    assert_eq!(scene.rects()[1].fill(), SHELL_PALETTE.surface_raised);
}

#[test]
fn toolbar_buttons_publish_accessible_labels_and_a_toolbar_parent() {
    let context = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(7));
    let mut text_layout = TextInputLayoutEngine::new();
    let dispatch = UiDispatch::default();
    let toolbar = InputContextToolbar::new(
        Rect::from_xywh(24.0, 600.0, 952.0, 24.0),
        &context,
        ComposerMode::Agent,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    );
    let mut frame = InteractionFrame::default();

    toolbar.register_interactions(&mut frame);

    let nodes = frame.accessibility_nodes(&dispatch);
    let location = nodes
        .iter()
        .find(|node| node.id == ContextAction::Location.element_id())
        .unwrap();
    assert_eq!(
        location.parent,
        Some(crate::shell_interaction::CONTEXT_TOOLBAR)
    );
    assert_eq!(location.label, "Environment: Local");
    assert!(location.focusable);
    let changes = nodes
        .iter()
        .find(|node| node.id == ContextAction::Diff.element_id())
        .unwrap();
    assert_eq!(changes.label, "Workspace Changes 7 • +7 -0");
}
