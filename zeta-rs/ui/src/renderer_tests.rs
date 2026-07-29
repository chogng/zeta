use super::{UiRenderError, validate_text_block};
use crate::{Color, Point, Size, TextBlock, TextStyle};

#[test]
fn rejects_non_positive_text_bounds() {
    let block = TextBlock::new(
        "invalid",
        Point::new(0.0, 0.0),
        Size::new(0.0, 20.0),
        TextStyle::new(14.0, Color::WHITE),
    );

    assert!(matches!(
        validate_text_block(3, &block),
        Err(UiRenderError::InvalidTextBlock {
            index: 3,
            reason: "bounds must be positive",
        })
    ));
}

#[test]
fn rejects_non_finite_text_metrics() {
    let block = TextBlock::new(
        "invalid",
        Point::new(0.0, 0.0),
        Size::new(100.0, 20.0),
        TextStyle::new(f32::NAN, Color::WHITE),
    );

    assert!(matches!(
        validate_text_block(1, &block),
        Err(UiRenderError::InvalidTextBlock {
            index: 1,
            reason: "coordinates and metrics must be finite",
        })
    ));
}
