use super::WindowMetrics;
use super::WindowOptions;
use super::WindowOptionsError;
use crate::Size;
use crate::window::LogicalSize;
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

#[test]
fn window_options_validate_all_configured_sizes() {
    let invalid = WindowOptions::new("invalid").with_inner_size(LogicalSize::new(0.0, 600.0));
    let inverted = WindowOptions::new("inverted")
        .with_min_inner_size(LogicalSize::new(800.0, 600.0))
        .with_max_inner_size(LogicalSize::new(640.0, 480.0));

    assert!(matches!(
        invalid.validate(),
        Err(WindowOptionsError::InvalidSize {
            field: "inner size"
        })
    ));
    assert_eq!(
        inverted.validate(),
        Err(WindowOptionsError::InvalidSizeRange)
    );
}

#[test]
fn window_options_accept_independent_initial_platform_policies() {
    let options = WindowOptions::new("tool")
        .with_inner_size(LogicalSize::new(900.0, 720.0))
        .with_min_inner_size(LogicalSize::new(320.0, 240.0))
        .with_max_inner_size(LogicalSize::new(1920.0, 1080.0))
        .with_visible(false)
        .with_active(false)
        .with_resizable(false)
        .with_maximized(true)
        .with_fullscreen(false);

    assert_eq!(options.validate(), Ok(()));
    assert!(!options.visible);
    assert!(!options.active);
    assert!(!options.resizable);
    assert!(options.maximized);
    assert!(!options.fullscreen);
}
