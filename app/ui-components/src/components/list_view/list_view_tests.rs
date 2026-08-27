use super::{ListContentPadding, ListView, VirtualListLayout};
use crate::{
    Color, PaintRect, Point, Rect, ScrollAxis, ScrollCommand, ScrollDelta, ScrollState,
    ScrollViewStyle, ScrollbarStyle, Size, UiScene,
};

fn style() -> ScrollViewStyle {
    ScrollViewStyle::new(ScrollbarStyle::new(
        Color::TRANSPARENT,
        Color::rgb(126, 126, 132),
    ))
}

#[test]
fn fixed_extent_layout_projects_visible_and_overscan_ranges() {
    let bounds = Rect::from_xywh(10.0, 20.0, 200.0, 100.0);
    let mut state = ScrollState::default();
    state.apply(
        ScrollCommand::ByPixels(ScrollDelta::vertical(60.0)),
        crate::ScrollMetrics::new(bounds.size, Size::new(200.0, 1_000.0)),
        ScrollAxis::Vertical,
    );
    let list = ListView::new(bounds, 50, 20.0, state, style()).with_overscan_items(2);

    assert_eq!(list.visible_range(), 3..8);
    assert_eq!(
        list.layout().projected_range(list.scroll_view().viewport()),
        1..10
    );
    assert_eq!(
        list.item_bounds(3),
        Some(Rect::from_xywh(10.0, 20.0, 200.0, 20.0))
    );
}

#[test]
fn item_hit_testing_and_ensure_visible_use_content_coordinates() {
    let bounds = Rect::from_xywh(10.0, 20.0, 200.0, 100.0);
    let list = ListView::new(bounds, 50, 20.0, ScrollState::default(), style());

    assert_eq!(list.item_at(Point::new(20.0, 65.0)), Some(2));
    assert_eq!(list.item_at(Point::new(20.0, 125.0)), None);
    assert_eq!(
        list.ensure_visible_command(12),
        Some(ScrollCommand::EnsureVisible(Rect::from_xywh(
            0.0, 240.0, 200.0, 20.0
        )))
    );
    assert_eq!(list.ensure_visible_command(50), None);
}

#[test]
fn draw_only_invokes_projected_items_and_clips_them_to_the_viewport() {
    let bounds = Rect::from_xywh(0.0, 0.0, 100.0, 60.0);
    let list =
        ListView::new(bounds, 1_000, 20.0, ScrollState::default(), style()).with_overscan_items(1);
    let mut scene = UiScene::new(Color::WHITE);
    let mut indices = Vec::new();

    list.draw(&mut scene, |scene, item| {
        indices.push(item.index());
        scene.draw_rect(PaintRect::new(item.bounds(), Color::WHITE));
    });

    assert_eq!(indices, vec![0, 1, 2, 3]);
    assert_eq!(scene.rects().len(), 5);
    assert!(
        scene
            .rects()
            .iter()
            .take(3)
            .all(|rect| rect.clip_bounds() == Some(bounds))
    );
}

#[test]
fn empty_layout_has_no_visible_or_hit_testable_items() {
    let bounds = Rect::from_xywh(0.0, 0.0, 100.0, 60.0);
    let layout = VirtualListLayout::new(0, 20.0);
    let viewport = ListView::new(bounds, 0, 20.0, ScrollState::default(), style())
        .scroll_view()
        .viewport();

    assert_eq!(layout.visible_range(viewport), 0..0);
    assert_eq!(layout.item_at(Point::new(10.0, 10.0), viewport), None);
}

#[test]
fn variable_extent_layout_uses_prefix_geometry_and_skips_gaps() {
    let bounds = Rect::from_xywh(10.0, 20.0, 200.0, 50.0);
    let layout = VirtualListLayout::variable([20.0, 60.0, 30.0])
        .with_item_gap(5.0)
        .with_content_padding(ListContentPadding::new(10.0, 15.0))
        .with_overscan_items(1);
    let mut state = ScrollState::default();
    state.apply(
        ScrollCommand::ByPixels(ScrollDelta::vertical(40.0)),
        crate::ScrollMetrics::new(bounds.size, layout.content_size(bounds.size.width)),
        ScrollAxis::Vertical,
    );
    let list = ListView::from_layout(bounds, layout, state, style());

    assert_eq!(list.layout().content_extent(), 145.0);
    assert_eq!(list.visible_range(), 1..2);
    assert_eq!(
        list.layout().projected_range(list.scroll_view().viewport()),
        0..3
    );
    assert_eq!(
        list.item_bounds(1),
        Some(Rect::from_xywh(10.0, 15.0, 200.0, 60.0))
    );
    assert_eq!(list.item_at(Point::new(20.0, 20.0)), Some(1));

    let unscrolled = ListView::from_layout(
        bounds,
        list.layout().clone(),
        ScrollState::default(),
        style(),
    );
    assert_eq!(unscrolled.item_at(Point::new(20.0, 52.0)), None);
    assert_eq!(
        unscrolled.ensure_visible_command(2),
        Some(ScrollCommand::EnsureVisible(Rect::from_xywh(
            0.0, 100.0, 200.0, 30.0
        )))
    );
}
