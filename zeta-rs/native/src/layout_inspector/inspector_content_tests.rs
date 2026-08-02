use super::{authored_layout, metrics};
use zeta_ui::{Color, Edges, Element, ElementLength, Rect, UiScene};

#[test]
fn separates_authored_layout_from_computed_box_metrics() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_element(
        Element::column("Panel")
            .width(ElementLength::px(240.0))
            .padding(Edges::uniform(80.0))
            .in_bounds(Rect::from_xywh(0.0, 0.0, 40.0, 30.0)),
        |_, _| {},
    );
    let node = &scene.inspection().nodes()[0];

    assert_eq!(authored_layout(node), "column   width 240   height fill");
    assert_eq!(metrics(node), "size 40 × 30   padding 30 0 0 40");
}
