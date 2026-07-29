use super::{Border, Color, PaintRect};
use crate::{CornerRadii, Edges, Rect};

#[test]
fn paint_rect_retains_explicit_visual_properties() {
    let bounds = Rect::from_xywh(4.0, 8.0, 120.0, 32.0);
    let border = Border::new(Edges::new(1.0, 2.0, 3.0, 4.0), Color::WHITE);
    let rect = PaintRect::new(bounds, Color::rgb(10, 20, 30))
        .with_border(border)
        .with_corner_radii(CornerRadii::uniform(6.0));

    assert_eq!(rect.bounds(), bounds);
    assert_eq!(rect.border(), border);
    assert_eq!(rect.corner_radii(), CornerRadii::uniform(6.0));
}

#[test]
fn applying_multiple_clips_intersects_them() {
    let mut rect = PaintRect::new(Rect::from_xywh(0.0, 0.0, 100.0, 100.0), Color::WHITE);

    rect.apply_clip(Rect::from_xywh(10.0, 10.0, 80.0, 80.0));
    rect.apply_clip(Rect::from_xywh(50.0, 0.0, 60.0, 60.0));

    assert_eq!(
        rect.clip_bounds(),
        Some(Rect::from_xywh(50.0, 10.0, 40.0, 50.0))
    );
}
