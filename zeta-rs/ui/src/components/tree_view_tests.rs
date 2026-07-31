use super::{TreeItem, TreeItemExpansion, TreeView, TreeViewStyle};
use crate::{
    Color, PaintRect, Point, Rect, ScrollAxis, ScrollCommand, ScrollDelta, ScrollState,
    ScrollViewStyle, ScrollbarStyle, Size, UiScene,
};

fn style() -> TreeViewStyle {
    TreeViewStyle::new(
        ScrollViewStyle::new(ScrollbarStyle::new(
            Color::TRANSPARENT,
            Color::rgb(126, 126, 132),
        )),
        24.0,
    )
}

#[test]
fn depth_and_expansion_produce_disclosure_and_content_geometry() {
    let items = [
        TreeItem::new(0, TreeItemExpansion::Expanded),
        TreeItem::new(1, TreeItemExpansion::Leaf),
        TreeItem::new(1, TreeItemExpansion::Collapsed),
    ];
    let tree = TreeView::new(
        Rect::from_xywh(10.0, 20.0, 200.0, 72.0),
        &items,
        ScrollState::default(),
        style(),
    );

    let root = tree.item_layout(0).unwrap();
    let leaf = tree.item_layout(1).unwrap();
    assert_eq!(
        root.disclosure_bounds(),
        Some(Rect::from_xywh(10.0, 24.0, 16.0, 16.0))
    );
    assert_eq!(
        root.content_bounds(),
        Rect::from_xywh(30.0, 20.0, 180.0, 24.0)
    );
    assert_eq!(leaf.disclosure_bounds(), None);
    assert_eq!(
        leaf.content_bounds(),
        Rect::from_xywh(42.0, 44.0, 168.0, 24.0)
    );
    assert_eq!(tree.disclosure_at(Point::new(18.0, 28.0)), Some(0));
    assert_eq!(tree.disclosure_at(Point::new(34.0, 52.0)), None);
}

#[test]
fn tree_draw_only_projects_virtualized_rows() {
    let items = (0..1_000)
        .map(|index| {
            TreeItem::new(
                index % 3,
                if index % 5 == 0 {
                    TreeItemExpansion::Collapsed
                } else {
                    TreeItemExpansion::Leaf
                },
            )
        })
        .collect::<Vec<_>>();
    let mut state = ScrollState::default();
    state.apply(
        ScrollCommand::ByPixels(ScrollDelta::vertical(48.0)),
        crate::ScrollMetrics::new(Size::new(200.0, 48.0), Size::new(200.0, 24_000.0)),
        ScrollAxis::Vertical,
    );
    let tree = TreeView::new(
        Rect::from_xywh(0.0, 0.0, 200.0, 48.0),
        &items,
        state,
        style(),
    )
    .with_overscan_items(1);
    let mut scene = UiScene::new(Color::WHITE);
    let mut projected = Vec::new();

    tree.draw(&mut scene, |scene, item| {
        projected.push(item.index());
        scene.draw_rect(PaintRect::new(item.bounds(), Color::WHITE));
    });

    assert_eq!(tree.visible_range(), 2..4);
    assert_eq!(projected, vec![1, 2, 3, 4]);
    assert!(scene.rects().len() < 10);
}
