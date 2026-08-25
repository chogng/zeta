use super::{srgb_channel, wgpu_color};
use crate::ui::foundation::Color;

#[test]
fn converts_srgb_background_to_linear_wgpu_color() {
    let color = wgpu_color(Color::rgba(128, 64, 255, 127));

    assert!((color.r - srgb_channel(128)).abs() < f64::EPSILON);
    assert!((color.g - srgb_channel(64)).abs() < f64::EPSILON);
    assert_eq!(color.b, 1.0);
    assert_eq!(color.a, 127.0 / 255.0);
}

#[test]
fn preserves_black_and_white_endpoints() {
    assert_eq!(srgb_channel(0), 0.0);
    assert_eq!(srgb_channel(255), 1.0);
}
