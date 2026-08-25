use super::WindowMetrics;
use crate::Size;
use crate::window::PhysicalExtent;

#[test]
fn logical_size_uses_valid_platform_scale_factor() {
    let metrics = WindowMetrics::new(PhysicalExtent::new(1440, 900), 2.0);

    assert_eq!(metrics.logical_size(), Size::new(720.0, 450.0));
}

#[test]
fn logical_size_falls_back_for_invalid_platform_scale_factor() {
    for scale_factor in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let metrics = WindowMetrics::new(PhysicalExtent::new(800, 600), scale_factor);

        assert_eq!(metrics.logical_size(), Size::new(800.0, 600.0));
        assert_eq!(metrics.scale_factor(), 1.0);
    }
}
