use super::{FontFamily, FontWeight, TextBlock, TextStyle, UiScene};
use crate::{Color, PaintIcon, PaintRect, Point, Rect, Size, SvgIcon};

const TEST_ICON: SvgIcon = SvgIcon::new(
    "test",
    br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6"/></svg>"#,
);

#[test]
fn scene_retains_text_in_paint_order() {
    let style = TextStyle::new(16.0, Color::WHITE)
        .with_family(FontFamily::Monospace)
        .with_weight(FontWeight::Bold);
    let first = TextBlock::new(
        "first",
        Point::new(8.0, 12.0),
        Size::new(200.0, 24.0),
        style.clone(),
    );
    let second = TextBlock::new(
        "second",
        Point::new(8.0, 40.0),
        Size::new(200.0, 24.0),
        style,
    );
    let mut scene = UiScene::new(Color::rgb(10, 20, 30));

    scene.draw_text(first.clone());
    scene.draw_text(second.clone());

    assert_eq!(scene.text_blocks(), &[first, second]);
    assert!(scene.rects().is_empty());
    assert!(scene.icons().is_empty());
    assert_eq!(scene.background().components(), [10, 20, 30, 255]);
}

#[test]
fn text_style_defaults_to_readable_line_height() {
    let style = TextStyle::new(20.0, Color::WHITE);

    assert_eq!(style.line_height(), 24.0);
    assert_eq!(style.family(), &FontFamily::SansSerif);
}

#[test]
fn scene_applies_nested_clip_to_all_primitives() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_clip(Rect::from_xywh(10.0, 10.0, 100.0, 80.0), |scene| {
        scene.with_clip(Rect::from_xywh(50.0, 0.0, 80.0, 60.0), |scene| {
            scene.draw_rect(PaintRect::new(
                Rect::from_xywh(0.0, 0.0, 200.0, 200.0),
                Color::WHITE,
            ));
            scene.draw_icon(PaintIcon::new(
                TEST_ICON,
                Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
                Color::WHITE,
            ));
            scene.draw_text(TextBlock::new(
                "clipped",
                Point::new(0.0, 0.0),
                Size::new(200.0, 40.0),
                TextStyle::new(14.0, Color::WHITE),
            ));
        });
    });

    let expected = Some(Rect::from_xywh(50.0, 10.0, 60.0, 50.0));
    assert_eq!(scene.rects()[0].clip_bounds(), expected);
    assert_eq!(scene.icons()[0].clip_bounds(), expected);
    assert_eq!(scene.text_blocks()[0].clip_bounds(), expected);
}

#[test]
fn scene_restores_outer_clip_after_nested_draw() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_clip(Rect::from_xywh(10.0, 10.0, 100.0, 80.0), |scene| {
        scene.with_clip(Rect::from_xywh(50.0, 0.0, 80.0, 60.0), |_| {});
        scene.draw_rect(PaintRect::new(
            Rect::from_xywh(0.0, 0.0, 200.0, 200.0),
            Color::WHITE,
        ));
    });

    assert_eq!(
        scene.rects()[0].clip_bounds(),
        Some(Rect::from_xywh(10.0, 10.0, 100.0, 80.0))
    );
}
