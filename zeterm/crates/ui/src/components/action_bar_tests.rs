use zeta_icons::icons;

use super::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarSeparatorStyle,
    ActionBarStyle,
};
use crate::{
    ButtonBackgrounds, ButtonSelection, ButtonState, ButtonStyle, Color, Component, Edges, Point,
    Rect, Size, TextStyle, UiScene,
};

fn test_style() -> ActionBarStyle {
    ActionBarStyle::new(
        ButtonStyle::new(
            ButtonBackgrounds::new(Color::rgb(10, 20, 30))
                .with_hovered(Color::rgb(20, 30, 40))
                .with_pressed(Color::rgb(30, 40, 50))
                .with_disabled(Color::rgb(5, 10, 15)),
            TextStyle::new(12.0, Color::WHITE),
        )
        .with_selected_backgrounds(
            ButtonBackgrounds::new(Color::rgb(40, 50, 60))
                .with_hovered(Color::rgb(50, 60, 70))
                .with_pressed(Color::rgb(60, 70, 80))
                .with_disabled(Color::rgb(20, 25, 30)),
        )
        .with_padding(Edges::uniform(2.0))
        .with_icon_size(8.0)
        .with_content_gap(2.0),
        Size::new(24.0, 20.0),
    )
    .with_gap(4.0)
    .with_separator_style(ActionBarSeparatorStyle::new(Color::rgb(90, 100, 110)))
}

#[test]
fn horizontal_action_bar_owns_button_and_separator_geometry() {
    let action_bar = ActionBar::new(
        Rect::from_xywh(10.0, 5.0, 100.0, 20.0),
        ActionBarOrientation::Horizontal,
        vec![
            ActionBarItem::Button(ActionBarButton::icon(
                icons::FILES,
                "Files",
                ButtonState::Resting,
            )),
            ActionBarItem::Separator,
            ActionBarItem::Button(ActionBarButton::label("Open", ButtonState::Hovered)),
        ],
        test_style(),
    );

    assert_eq!(
        action_bar.item_bounds(0),
        Some(Rect::from_xywh(10.0, 5.0, 24.0, 20.0))
    );
    assert_eq!(action_bar.item_bounds(1), None);
    assert_eq!(
        action_bar.item_bounds(2),
        Some(Rect::from_xywh(50.0, 5.0, 24.0, 20.0))
    );
    assert_eq!(action_bar.hit_test(Point::new(22.0, 15.0)), Some(0));
    assert_eq!(action_bar.hit_test(Point::new(38.0, 15.0)), None);
    assert_eq!(action_bar.hit_test(Point::new(60.0, 15.0)), Some(2));
}

#[test]
fn action_bar_paints_button_variants_and_noninteractive_separator() {
    let action_bar = ActionBar::new(
        Rect::from_xywh(0.0, 0.0, 100.0, 20.0),
        ActionBarOrientation::Horizontal,
        vec![
            ActionBarItem::Button(
                ActionBarButton::icon(icons::FILES, "Files", ButtonState::Resting)
                    .with_selection(ButtonSelection::Selected),
            ),
            ActionBarItem::Separator,
            ActionBarItem::Button(ActionBarButton::icon_and_label(
                icons::ADD,
                "Add",
                ButtonState::Disabled,
            )),
        ],
        test_style(),
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);

    action_bar.paint(&mut scene);

    assert_eq!(scene.rects().len(), 3);
    assert_eq!(scene.rects()[0].fill(), Color::rgb(40, 50, 60));
    assert_eq!(scene.rects()[1].fill(), Color::rgb(90, 100, 110));
    assert_eq!(scene.rects()[2].fill(), Color::rgb(5, 10, 15));
    assert_eq!(scene.icons().len(), 2);
    assert_eq!(scene.text_blocks()[0].text(), "Add");
    assert_eq!(
        action_bar.item_bounds(2),
        Some(Rect::from_xywh(40.0, 0.0, 24.0, 20.0))
    );
    assert_eq!(action_bar.interactive_item_bounds(2), None);
    assert_eq!(action_bar.hit_test(Point::new(50.0, 10.0)), None);
}

#[test]
fn vertical_action_bar_maps_item_extent_to_the_vertical_axis() {
    let action_bar = ActionBar::new(
        Rect::from_xywh(3.0, 7.0, 24.0, 80.0),
        ActionBarOrientation::Vertical,
        vec![
            ActionBarItem::Button(ActionBarButton::label("One", ButtonState::Resting)),
            ActionBarItem::Button(ActionBarButton::label("Two", ButtonState::Pressed)),
        ],
        test_style(),
    );

    assert_eq!(
        action_bar.item_bounds(0),
        Some(Rect::from_xywh(3.0, 7.0, 24.0, 20.0))
    );
    assert_eq!(
        action_bar.item_bounds(1),
        Some(Rect::from_xywh(3.0, 31.0, 24.0, 20.0))
    );
}

#[test]
fn action_bar_inspection_reports_the_resolved_item_gap() {
    let action_bar = ActionBar::new(
        Rect::from_xywh(0.0, 0.0, 100.0, 20.0),
        ActionBarOrientation::Horizontal,
        vec![
            ActionBarItem::Button(ActionBarButton::label("One", ButtonState::Resting)),
            ActionBarItem::Separator,
            ActionBarItem::Button(ActionBarButton::label("Two", ButtonState::Resting)),
        ],
        test_style(),
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);

    scene.draw_component(&action_bar);

    let node = scene
        .inspection()
        .nodes()
        .iter()
        .find(|node| node.name() == "ActionBar")
        .expect("ActionBar inspection node");
    assert_eq!(node.gap(), Some(4.0));
    assert_eq!(
        node.gap_regions(),
        &[
            Rect::from_xywh(24.0, 0.0, 4.0, 20.0),
            Rect::from_xywh(36.0, 0.0, 4.0, 20.0),
        ]
    );
}

#[test]
fn action_buttons_can_override_their_main_axis_extent() {
    let action_bar = ActionBar::new(
        Rect::from_xywh(10.0, 5.0, 140.0, 20.0),
        ActionBarOrientation::Horizontal,
        vec![
            ActionBarItem::Button(
                ActionBarButton::label("Short", ButtonState::Resting).with_main_axis_extent(40.0),
            ),
            ActionBarItem::Button(
                ActionBarButton::label("Long", ButtonState::Resting).with_main_axis_extent(72.0),
            ),
        ],
        test_style(),
    );

    assert_eq!(
        action_bar.item_bounds(0),
        Some(Rect::from_xywh(10.0, 5.0, 40.0, 20.0))
    );
    assert_eq!(
        action_bar.item_bounds(1),
        Some(Rect::from_xywh(54.0, 5.0, 72.0, 20.0))
    );
}
