use super::{Dropdown, DropdownItem, DropdownSelection, DropdownStyle};
use crate::{
    ButtonBackgrounds, ButtonState, ButtonStyle, Color, Component, CornerRadii, Point, Rect, Size,
    TextStyle, UiScene,
};

const SURFACE: Color = Color::rgb(240, 241, 242);
const SELECTED: Color = Color::rgb(210, 211, 212);

fn dropdown(items: Vec<DropdownItem>) -> Dropdown {
    let button_style = ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT),
        TextStyle::new(13.0, Color::rgb(0, 0, 0)),
    )
    .with_selected_backgrounds(ButtonBackgrounds::new(SELECTED));
    Dropdown::new(
        Rect::from_xywh(0.0, 0.0, 400.0, 300.0),
        Rect::from_xywh(30.0, 20.0, 1.0, 1.0),
        items,
        DropdownStyle::new(SURFACE, button_style, Size::new(120.0, 28.0))
            .with_corner_radii(CornerRadii::uniform(6.0)),
    )
}

#[test]
fn defaults_to_the_first_enabled_item() {
    let dropdown = dropdown(vec![
        DropdownItem::new("Disabled", ButtonState::Disabled),
        DropdownItem::new("First enabled", ButtonState::Resting),
        DropdownItem::new("Last", ButtonState::Resting),
    ]);
    let mut scene = UiScene::new(Color::TRANSPARENT);

    dropdown.paint(&mut scene);

    assert_eq!(dropdown.selected_index(), Some(1));
    assert_eq!(scene.rects()[2].fill(), SELECTED);
}

#[test]
fn borderless_surface_has_no_outer_padding() {
    let dropdown = dropdown(vec![
        DropdownItem::new("Pin", ButtonState::Resting),
        DropdownItem::new("Close", ButtonState::Resting),
    ]);
    let mut scene = UiScene::new(Color::TRANSPARENT);

    dropdown.paint(&mut scene);

    assert_eq!(dropdown.bounds(), dropdown.content_bounds());
    assert_eq!(
        dropdown.item_bounds(0).unwrap().origin,
        dropdown.bounds().origin
    );
    assert_eq!(
        dropdown.item_bounds(0).unwrap().size.width,
        dropdown.bounds().size.width
    );
    assert_eq!(
        scene.rects()[0].border().widths(),
        crate::Edges::uniform(0.0)
    );
}

#[test]
fn explicit_selection_drives_paint_and_hit_geometry() {
    let dropdown = dropdown(vec![
        DropdownItem::new("Pin", ButtonState::Resting),
        DropdownItem::new("Close", ButtonState::Focused),
    ])
    .with_selection(DropdownSelection::Item(1));
    let selected_bounds = dropdown.item_bounds(1).unwrap();
    let mut scene = UiScene::new(Color::TRANSPARENT);

    dropdown.paint(&mut scene);

    assert_eq!(dropdown.selected_index(), Some(1));
    assert_eq!(scene.rects()[2].fill(), SELECTED);
    assert_eq!(
        dropdown.hit_test(Point::new(
            selected_bounds.origin.x + 1.0,
            selected_bounds.origin.y + 1.0
        )),
        Some(1)
    );
}
