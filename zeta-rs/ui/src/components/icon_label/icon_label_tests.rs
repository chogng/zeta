use zeta_icons::icons;

use super::{IconLabel, IconLabelStyle};
use crate::{Color, Component, Rect, TextStyle, UiScene};

#[test]
fn icon_label_aligns_semantic_icon_and_text_inside_its_bounds() {
    let label = IconLabel::new(
        Rect::from_xywh(20.0, 10.0, 140.0, 28.0),
        icons::FILES,
        "Files",
        IconLabelStyle::new(TextStyle::new(13.0, Color::rgb(90, 120, 150)))
            .with_icon_size(16.0)
            .with_content_gap(6.0),
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);

    label.paint(&mut scene);

    assert_eq!(scene.icons()[0].icon(), icons::FILES);
    assert_eq!(
        scene.icons()[0].bounds(),
        Rect::from_xywh(20.0, 16.0, 16.0, 16.0)
    );
    assert_eq!(scene.icons()[0].color(), Color::rgb(90, 120, 150));
    assert_eq!(scene.text_blocks()[0].origin().x, 42.0);
    assert_eq!(scene.text_blocks()[0].text(), "Files");
}

#[test]
fn icon_label_skips_content_when_bounds_are_empty() {
    let label = IconLabel::new(
        Rect::from_xywh(0.0, 0.0, 0.0, 20.0),
        icons::FILES,
        "Files",
        IconLabelStyle::new(TextStyle::new(13.0, Color::WHITE)),
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);

    label.paint(&mut scene);

    assert!(scene.icons().is_empty());
    assert!(scene.text_blocks().is_empty());
}
