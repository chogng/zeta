use super::{Element, ElementLength};
use crate::{CornerRadii, Edges, Rect};

#[test]
fn row_resolves_padding_fixed_fill_children_and_exact_gap_regions() {
    let layout = Element::row("Toolbar")
        .padding(Edges::new(2.0, 3.0, 4.0, 5.0))
        .gap(6.0)
        .corner_radii(CornerRadii::uniform(8.0))
        .child(
            Element::row("Fixed")
                .width(ElementLength::px(20.0))
                .height(ElementLength::px(10.0)),
        )
        .child(Element::row("Fill").height(ElementLength::px(12.0)))
        .in_bounds(Rect::from_xywh(10.0, 20.0, 100.0, 30.0))
        .compute();

    assert_eq!(
        layout.children()[0].bounds(),
        Rect::from_xywh(15.0, 22.0, 20.0, 10.0)
    );
    assert_eq!(
        layout.children()[1].bounds(),
        Rect::from_xywh(41.0, 22.0, 66.0, 12.0)
    );
    assert_eq!(
        layout.gap_regions(),
        &[Rect::from_xywh(35.0, 22.0, 6.0, 24.0)]
    );
    let inspection = layout.inspection_node();
    assert_eq!(inspection.padding(), Some(Edges::new(2.0, 3.0, 4.0, 5.0)));
    assert_eq!(inspection.gap(), Some(6.0));
    assert_eq!(inspection.gap_regions(), layout.gap_regions());
    assert_eq!(inspection.corner_radii(), Some(CornerRadii::uniform(8.0)));
}

#[test]
fn column_clips_children_and_gap_regions_to_the_available_bounds() {
    let layout = Element::column("List")
        .gap(6.0)
        .children([
            Element::row("One").height(ElementLength::px(20.0)),
            Element::row("Two").height(ElementLength::px(20.0)),
            Element::row("Three").height(ElementLength::px(20.0)),
        ])
        .in_bounds(Rect::from_xywh(0.0, 0.0, 40.0, 45.0))
        .compute();

    assert_eq!(
        layout.children()[0].bounds(),
        Rect::from_xywh(0.0, 0.0, 40.0, 20.0)
    );
    assert_eq!(
        layout.children()[1].bounds(),
        Rect::from_xywh(0.0, 26.0, 40.0, 19.0)
    );
    assert!(layout.children()[2].bounds().is_empty());
    assert_eq!(
        layout.gap_regions(),
        &[Rect::from_xywh(0.0, 20.0, 40.0, 6.0)]
    );
}
