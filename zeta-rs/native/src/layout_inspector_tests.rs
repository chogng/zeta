use super::LayoutInspector;
use crate::root_layout::{InspectorPane, RootLayout};
use crate::shell_scene::LogicalViewport;
use zeta_icons::icons;
use zeta_ui::{Color, CornerRadii, Edges, InspectionNode, Point, Rect, UiScene};

#[test]
fn decoration_paints_the_full_ancestry_in_the_added_panel() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_inspection_node(
        InspectionNode::new("Toolbar", Rect::from_xywh(0.0, 0.0, 300.0, 48.0)).with_gap(6.0),
        |scene| {
            scene.with_inspection_node(
                InspectionNode::new("InputBox", Rect::from_xywh(10.0, 8.0, 120.0, 32.0))
                    .with_padding(Edges::uniform(8.0))
                    .with_corner_radii(CornerRadii::uniform(4.0)),
                |_| {},
            );
        },
    );
    let mut inspector = LayoutInspector::default();
    inspector.open(400.0);
    inspector.toggle_picking();

    inspector.compose(
        &mut scene,
        inspector_root_layout(400.0, 300.0),
        Some(Point::new(20.0, 20.0)),
    );

    let text = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    let inspected_names = scene
        .inspection()
        .nodes()
        .iter()
        .map(InspectionNode::name)
        .collect::<Vec<_>>();
    assert!(inspected_names.contains(&"Toolbar"));
    assert!(inspected_names.contains(&"InputBox"));
    assert!(inspected_names.contains(&"InspectorPanel"));
    assert!(inspected_names.contains(&"InspectorToolbar"));
    assert!(inspected_names.contains(&"InspectorContent"));
    assert!(inspected_names.contains(&"ActionBar"));
    assert!(inspected_names.contains(&"Button"));
    for node in scene.inspection().nodes().iter().filter(|node| {
        matches!(
            node.name(),
            "InspectorPanel" | "InspectorToolbar" | "InspectorContent"
        )
    }) {
        assert_eq!(node.layer(), 0);
    }
    assert!(scene.rect_layers().contains(&1));
    assert!(!text.iter().any(|value| value.contains("Layout Inspector")));
    assert!(
        !text
            .iter()
            .any(|value| value.contains("Use the cursor tool"))
    );
    assert!(text.iter().any(|value| value.contains("Toolbar")));
    assert!(text.iter().any(|value| value.contains("InputBox")));
    assert!(text.iter().any(|value| value.contains("gap 6")));
    assert!(
        scene
            .icons()
            .iter()
            .any(|icon| icon.icon() == icons::CURSOR_FILLED)
    );
    assert!(scene.icons().iter().any(|icon| icon.icon() == icons::CLOSE));
    assert!(
        text.iter()
            .any(|value| value.contains("size 120 × 32") && value.contains("padding 8 8 8 8"))
    );
}

#[test]
fn content_width_stays_fixed_until_the_inspector_window_closes() {
    let mut inspector = LayoutInspector::default();
    inspector.open(1_000.0);
    let mut scene = UiScene::new(Color::TRANSPARENT);
    inspector.compose(
        &mut scene,
        inspector_root_layout(1_000.0, 700.0),
        Some(Point::new(20.0, 40.0)),
    );

    assert_eq!(
        inspector.content_viewport(LogicalViewport {
            width: 1_360.0,
            height: 700.0,
        }),
        LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        }
    );

    assert_eq!(inspector.close(), Some(1_000.0));
    assert_eq!(
        inspector.content_viewport(LogicalViewport {
            width: 1_360.0,
            height: 700.0,
        }),
        LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        }
    );

    inspector.window_resized(LogicalViewport {
        width: 1_000.0,
        height: 700.0,
    });
    assert_eq!(
        inspector.content_viewport(LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        }),
        LogicalViewport {
            width: 1_000.0,
            height: 700.0,
        }
    );
}

#[test]
fn inspection_cursor_is_limited_to_the_application_content() {
    let mut inspector = LayoutInspector::default();
    inspector.open(1_000.0);
    let mut scene = UiScene::new(Color::TRANSPARENT);
    inspector.compose(
        &mut scene,
        inspector_root_layout(1_000.0, 700.0),
        Some(Point::new(20.0, 40.0)),
    );

    assert!(!inspector.uses_inspection_cursor(Some(Point::new(20.0, 40.0))));
    assert!(!inspector.pointer_is_over_panel(Some(Point::new(999.0, 40.0))));
    assert!(inspector.pointer_is_over_panel(Some(Point::new(1_000.0, 40.0))));
    inspector.toggle_picking();
    assert!(inspector.uses_inspection_cursor(Some(Point::new(999.0, 40.0))));
    assert!(!inspector.uses_inspection_cursor(Some(Point::new(1_000.0, 40.0))));
    assert!(!inspector.uses_inspection_cursor(Some(Point::new(1_200.0, 40.0))));
    assert!(!inspector.uses_inspection_cursor(None));

    inspector.close();
    assert!(!inspector.uses_inspection_cursor(Some(Point::new(20.0, 40.0))));
    assert!(!inspector.pointer_is_over_panel(Some(Point::new(1_200.0, 40.0))));
}

#[test]
fn selecting_a_component_stops_picking_and_retains_the_selection() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_inspection_node(
        InspectionNode::new("Toolbar", Rect::from_xywh(0.0, 0.0, 300.0, 48.0)),
        |_| {},
    );
    let mut inspector = LayoutInspector::default();
    inspector.open(400.0);
    inspector.toggle_picking();

    inspector.select(super::selection_at(&scene, Point::new(20.0, 20.0)));

    assert!(!inspector.is_picking());
    assert_eq!(
        inspector
            .locked
            .as_ref()
            .and_then(|selection| selection.target())
            .map(InspectionNode::name),
        Some("Toolbar")
    );
}

#[test]
fn panel_rows_retarget_the_selection_without_discarding_descendants() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_inspection_node(
        InspectionNode::new("ComposerPanel", Rect::from_xywh(0.0, 0.0, 300.0, 100.0)),
        |scene| {
            scene.with_inspection_node(
                InspectionNode::new("ComposerEditor", Rect::from_xywh(10.0, 10.0, 280.0, 60.0)),
                |_| {},
            );
        },
    );
    let mut inspector = LayoutInspector::default();
    inspector.open(400.0);
    inspector.select(super::selection_at(&scene, Point::new(20.0, 20.0)));
    inspector.compose(
        &mut scene,
        inspector_root_layout(400.0, 300.0),
        Some(Point::new(420.0, 70.0)),
    );

    assert!(inspector.uses_panel_action_cursor(Some(Point::new(420.0, 70.0))));
    assert!(inspector.select_panel_row(Point::new(420.0, 70.0)));

    let selection = inspector.locked.as_ref().expect("selection remains locked");
    assert_eq!(selection.path.len(), 2);
    assert_eq!(
        selection.target().map(InspectionNode::name),
        Some("ComposerPanel")
    );
    let mut overlay = UiScene::new(Color::TRANSPARENT);
    super::paint_selection(&mut overlay, selection);
    assert_eq!(overlay.rects().len(), 1);
}

#[test]
fn selection_paints_exact_gap_regions_with_the_gap_color() {
    let gap_bounds = Rect::from_xywh(24.0, 0.0, 6.0, 24.0);
    let selection = super::InspectionSelection::new(vec![
        InspectionNode::new("ActionBar", Rect::from_xywh(0.0, 0.0, 100.0, 24.0))
            .with_gap_geometry(6.0, vec![gap_bounds]),
    ]);
    let mut overlay = UiScene::new(Color::TRANSPARENT);

    super::paint_selection(&mut overlay, &selection);

    assert_eq!(overlay.rects().len(), 2);
    assert_eq!(overlay.rects()[0].bounds(), gap_bounds);
    assert_eq!(overlay.rects()[0].fill(), super::GAP_COLOR);
}

#[test]
fn inspector_toolbar_places_pick_and_close_actions_on_opposite_sides() {
    use super::inspector_toolbar::InspectorToolbarAction;

    let panel_bounds = Rect::from_xywh(1_000.0, 0.0, super::PANEL_WIDTH, 700.0);
    assert_eq!(
        super::panel::toolbar_action_at(panel_bounds, Point::new(1_010.0, 10.0)),
        Some(InspectorToolbarAction::Pick)
    );
    assert_eq!(
        super::panel::toolbar_action_at(panel_bounds, Point::new(1_340.0, 10.0)),
        Some(InspectorToolbarAction::Close)
    );
    assert_eq!(
        super::panel::toolbar_action_at(panel_bounds, Point::new(1_180.0, 10.0)),
        None
    );
    assert_eq!(
        super::panel::row_index_at(
            panel_bounds,
            Point::new(1_010.0, crate::titlebar::TITLEBAR_HEIGHT - 0.5),
            1,
        ),
        None
    );
    assert_eq!(
        super::panel::row_index_at(
            panel_bounds,
            Point::new(1_010.0, crate::titlebar::TITLEBAR_HEIGHT),
            1,
        ),
        Some(0)
    );
}

fn inspector_root_layout(product_width: f32, height: f32) -> RootLayout {
    RootLayout::for_viewports(
        LogicalViewport {
            width: product_width + super::PANEL_WIDTH,
            height,
        },
        LogicalViewport {
            width: product_width,
            height,
        },
        InspectorPane::visible(super::PANEL_WIDTH),
    )
}
