use super::LayoutInspector;
use crate::shell_scene::LogicalViewport;
use zeta_icons::icons;
use zeta_ui::{Color, CornerRadii, Edges, InspectionNode, Point, Rect, UiScene};

#[test]
fn decoration_paints_the_full_ancestry_in_the_added_panel() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_inspection_node(
        InspectionNode::new("Toolbar", Rect::from_xywh(0.0, 0.0, 300.0, 48.0)),
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

    inspector.decorate(
        &mut scene,
        LogicalViewport {
            width: 760.0,
            height: 300.0,
        },
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
    assert!(inspected_names.contains(&"Button"));
    assert!(text.iter().any(|value| value.contains("Layout Inspector")));
    assert!(text.iter().any(|value| value.contains("Toolbar")));
    assert!(text.iter().any(|value| value.contains("InputBox")));
    assert!(
        scene
            .icons()
            .iter()
            .any(|icon| icon.icon() == icons::CURSOR_FILLED)
    );
    assert!(
        text.iter()
            .any(|value| value.contains("size 120 × 32") && value.contains("padding 8 8 8 8"))
    );
}

#[test]
fn content_width_stays_fixed_until_the_inspector_window_closes() {
    let mut inspector = LayoutInspector::default();
    inspector.open(1_000.0);

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
fn picker_action_lives_inside_the_inspector_panel() {
    let bounds = super::panel::picker_bounds(1_000.0);

    assert!(bounds.contains(Point::new(1_010.0, 10.0)));
    assert!(!bounds.contains(Point::new(999.0, 10.0)));
}
