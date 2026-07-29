use super::{Color, FontFamily, FontWeight, Point, Size, TextBlock, TextStyle, UiScene};

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
    assert_eq!(scene.background().components(), [10, 20, 30, 255]);
}

#[test]
fn text_style_defaults_to_readable_line_height() {
    let style = TextStyle::new(20.0, Color::WHITE);

    assert_eq!(style.line_height(), 24.0);
    assert_eq!(style.family(), &FontFamily::SansSerif);
}
