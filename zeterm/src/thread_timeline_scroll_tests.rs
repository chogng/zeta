use super::ThreadTimelineScroll;
use zeta_winit::{MouseScrollDelta, PhysicalPosition};

#[test]
fn timeline_scroll_uses_bottom_relative_line_offsets() {
    let mut scroll = ThreadTimelineScroll::default();

    assert!(scroll.scroll(MouseScrollDelta::LineDelta(0.0, 2.0), 20));
    assert_eq!(scroll.offset(), 6);
    assert!(scroll.scroll(MouseScrollDelta::LineDelta(0.0, -1.0), 20));
    assert_eq!(scroll.offset(), 3);
}

#[test]
fn timeline_scroll_accumulates_trackpad_pixels() {
    let mut scroll = ThreadTimelineScroll::default();

    assert!(!scroll.scroll(
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 8.0)),
        20,
    ));
    assert!(scroll.scroll(
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 14.0)),
        20,
    ));
    assert_eq!(scroll.offset(), 1);
}
