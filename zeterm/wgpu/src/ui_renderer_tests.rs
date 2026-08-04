use super::{UiRenderError, glyphon_wrap, same_text_buffer_layout, validate_text_block};
use zui::{Color, Point, Size, TextBlock, TextBlockWrap, TextSpan, TextStyle};

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

#[test]
fn rejects_invalid_rich_text_span_metrics() {
    let base = TextStyle::new(14.0, Color::WHITE);
    let block = TextBlock::from_spans(
        [TextSpan::new("invalid", TextStyle::new(-1.0, Color::WHITE))],
        Point::new(0.0, 0.0),
        Size::new(100.0, 20.0),
        base,
    );

    assert!(matches!(
        validate_text_block(2, &block),
        Err(UiRenderError::InvalidTextBlock {
            index: 2,
            reason: "span font size and line height must be finite and positive",
        })
    ));
}

#[test]
fn renderer_preserves_the_text_blocks_explicit_wrap_contract() {
    assert_eq!(
        glyphon_wrap(TextBlockWrap::WordOrGlyph),
        glyphon::Wrap::WordOrGlyph
    );
    assert_eq!(glyphon_wrap(TextBlockWrap::None), glyphon::Wrap::None);
}

#[test]
fn text_buffer_layout_ignores_position_clip_and_default_color() {
    let first = TextBlock::new(
        "cached",
        Point::new(10.0, 20.0),
        Size::new(100.0, 24.0),
        TextStyle::new(14.0, Color::WHITE),
    );
    let moved = TextBlock::new(
        "cached",
        Point::new(90.0, 120.0),
        Size::new(100.0, 24.0),
        TextStyle::new(14.0, Color::rgb(20, 30, 40)),
    );

    assert!(same_text_buffer_layout(&first, &moved));
}

#[test]
fn text_buffer_layout_invalidates_shape_affecting_changes() {
    let original = TextBlock::new(
        "cached",
        Point::new(10.0, 20.0),
        Size::new(100.0, 24.0),
        TextStyle::new(14.0, Color::WHITE),
    );
    let resized = TextBlock::new(
        "cached",
        Point::new(10.0, 20.0),
        Size::new(80.0, 24.0),
        TextStyle::new(14.0, Color::WHITE),
    );
    let edited = TextBlock::new(
        "changed",
        Point::new(10.0, 20.0),
        Size::new(100.0, 24.0),
        TextStyle::new(14.0, Color::WHITE),
    );

    assert!(!same_text_buffer_layout(&original, &resized));
    assert!(!same_text_buffer_layout(&original, &edited));
}
