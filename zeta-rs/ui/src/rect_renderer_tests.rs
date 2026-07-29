use super::{linear_color, prepare_instances, validate_paint_rect};
use crate::{
    Border, Color, CornerRadii, Edges, PaintRect, Rect, UiRenderError, UiScene, UiViewport,
};

#[test]
fn prepares_logical_rect_in_physical_pixels() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_clip(Rect::from_xywh(5.0, 6.0, 70.0, 40.0), |scene| {
        scene.draw_rect(
            PaintRect::new(
                Rect::from_xywh(10.0, 12.0, 50.0, 20.0),
                Color::rgb(128, 64, 32),
            )
            .with_border(Border::new(Edges::new(1.0, 2.0, 3.0, 4.0), Color::WHITE))
            .with_corner_radii(CornerRadii::uniform(5.0)),
        );
    });

    let instances = prepare_instances(&scene, UiViewport::new(200, 120, 2.0)).unwrap();

    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].bounds, [20.0, 24.0, 100.0, 40.0]);
    assert_eq!(instances[0].border_widths, [2.0, 4.0, 6.0, 8.0]);
    assert_eq!(instances[0].corner_radii, [10.0; 4]);
    assert_eq!(instances[0].clip_bounds, [10.0, 12.0, 140.0, 80.0]);
}

#[test]
fn skips_rect_outside_empty_clip() {
    let mut scene = UiScene::new(Color::TRANSPARENT);
    scene.with_clip(Rect::from_xywh(200.0, 200.0, 10.0, 10.0), |scene| {
        scene.draw_rect(PaintRect::new(
            Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
            Color::WHITE,
        ));
    });

    let instances = prepare_instances(&scene, UiViewport::new(100, 100, 1.0)).unwrap();

    assert!(instances.is_empty());
}

#[test]
fn rejects_negative_border_width() {
    let rect = PaintRect::new(Rect::from_xywh(0.0, 0.0, 20.0, 20.0), Color::WHITE)
        .with_border(Border::new(Edges::new(1.0, -1.0, 1.0, 1.0), Color::WHITE));

    assert!(matches!(
        validate_paint_rect(4, rect),
        Err(UiRenderError::InvalidPaintRect {
            index: 4,
            reason: "border widths and corner radii must not be negative",
        })
    ));
}

#[test]
fn rejects_non_finite_requested_corner_radius() {
    let rect = PaintRect::new(Rect::from_xywh(0.0, 0.0, 20.0, 20.0), Color::WHITE)
        .with_corner_radii(CornerRadii::uniform(f32::NAN));

    assert!(matches!(
        validate_paint_rect(2, rect),
        Err(UiRenderError::InvalidPaintRect {
            index: 2,
            reason: "coordinates and visual metrics must be finite",
        })
    ));
}

#[test]
fn converts_srgb_channels_to_linear_values() {
    let converted = linear_color(Color::rgba(128, 255, 0, 128));

    assert!((converted[0] - 0.21586).abs() < 0.0001);
    assert_eq!(converted[1], 1.0);
    assert_eq!(converted[2], 0.0);
    assert!((converted[3] - 0.50196).abs() < 0.0001);
}
