use super::{InspectionFrame, InspectionNode};
use crate::{CornerRadii, Edges, Point, Rect};

#[test]
fn reverse_hit_test_prefers_the_deepest_recent_node() {
    let mut frame = InspectionFrame::default();
    let parent = frame.register(
        InspectionNode::new("Parent", Rect::from_xywh(0.0, 0.0, 100.0, 80.0)),
        None,
        0,
        "parent.rs",
        10,
    );
    let child = frame.register(
        InspectionNode::new("Child", Rect::from_xywh(10.0, 10.0, 40.0, 30.0)),
        Some(parent),
        0,
        "child.rs",
        20,
    );

    assert_eq!(
        frame
            .target_at(Point::new(20.0, 20.0))
            .map(|node| node.id()),
        Some(child)
    );
    assert_eq!(
        frame
            .ancestry(child)
            .iter()
            .map(|node| node.name())
            .collect::<Vec<_>>(),
        vec!["Parent", "Child"]
    );
}

#[test]
fn hit_test_prefers_an_overlay_even_when_base_content_registers_later() {
    let mut frame = InspectionFrame::default();
    let overlay = frame.register(
        InspectionNode::new("Overlay", Rect::from_xywh(0.0, 0.0, 40.0, 40.0)),
        None,
        1,
        "overlay.rs",
        10,
    );
    frame.register(
        InspectionNode::new("LaterBase", Rect::from_xywh(0.0, 0.0, 40.0, 40.0)),
        None,
        0,
        "base.rs",
        20,
    );

    assert_eq!(
        frame
            .target_at(Point::new(10.0, 10.0))
            .map(|node| node.id()),
        Some(overlay)
    );
}

#[test]
fn exposes_width_height_padding_and_resolved_radius() {
    let mut frame = InspectionFrame::default();
    let id = frame.register(
        InspectionNode::new("Button", Rect::from_xywh(2.0, 3.0, 20.0, 10.0))
            .with_padding(Edges::new(1.0, 2.0, 3.0, 4.0))
            .with_corner_radii(CornerRadii::uniform(8.0)),
        None,
        2,
        "button.rs",
        42,
    );
    let node = frame.node(id).expect("registered node");

    assert_eq!(node.width(), 20.0);
    assert_eq!(node.height(), 10.0);
    assert_eq!(node.padding(), Some(Edges::new(1.0, 2.0, 3.0, 4.0)));
    assert_eq!(node.corner_radii(), Some(CornerRadii::uniform(5.0)));
    assert_eq!(node.layer(), 2);
    assert_eq!(node.source_file(), "button.rs");
    assert_eq!(node.source_line(), 42);
}
