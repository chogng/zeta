use super::{IconLabel, IconLabelStyle};
use crate::{Color, Component, Rect, TextStyle, UiScene};
use zui::{Icon, IconDefinition, IconId};

const TEST_ICON: Icon = Icon::new(
    IconId::new("label"),
    IconDefinition::symbolic(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M3 3h10v10H3z"/></svg>"#,
    ),
);

#[test]
fn icon_label_aligns_semantic_icon_and_text_inside_its_bounds() {
    let label = IconLabel::new(
        Rect::from_xywh(20.0, 10.0, 140.0, 28.0),
        TEST_ICON,
        "Files",
        IconLabelStyle::new(TextStyle::new(13.0, Color::rgb(90, 120, 150)))
            .with_icon_size(16.0)
            .with_content_gap(6.0),
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);

    label.paint(&mut scene);

    assert_eq!(scene.icons()[0].icon(), TEST_ICON);
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
        TEST_ICON,
        "Files",
        IconLabelStyle::new(TextStyle::new(13.0, Color::WHITE)),
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);

    label.paint(&mut scene);

    assert!(scene.icons().is_empty());
    assert!(scene.text_blocks().is_empty());
}
