use super::{ContextMenu, ContextMenuItem, ContextMenuSelection, ContextMenuStyle};
use crate::{
    BoxShadow, ButtonBackgrounds, ButtonState, ButtonStyle, Color, Component, CornerRadii, Point,
    Rect, Size, TextStyle, UiScene,
};

const SURFACE: Color = Color::rgb(240, 241, 242);
const SELECTED: Color = Color::rgb(210, 211, 212);

fn context_menu(items: Vec<ContextMenuItem>) -> ContextMenu {
    let button_style = ButtonStyle::new(
        ButtonBackgrounds::new(Color::TRANSPARENT),
        TextStyle::new(13.0, Color::rgb(0, 0, 0)),
    )
    .with_selected_backgrounds(ButtonBackgrounds::new(SELECTED));
    ContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 400.0, 300.0),
        Rect::from_xywh(30.0, 20.0, 1.0, 1.0),
        items,
        ContextMenuStyle::new(SURFACE, button_style, Size::new(120.0, 28.0)),
    )
}

#[test]
fn owns_soft_shadow_two_pixel_padding_and_four_pixel_radius() {
    let menu = context_menu(vec![
        ContextMenuItem::new("Pin", ButtonState::Resting),
        ContextMenuItem::new("Close", ButtonState::Resting),
    ]);
    let mut scene = UiScene::new(Color::TRANSPARENT);

    menu.paint(&mut scene);

    assert_eq!(
        menu.content_bounds(),
        Rect::from_xywh(
            menu.bounds().origin.x + 2.0,
            menu.bounds().origin.y + 2.0,
            menu.bounds().size.width - 4.0,
            menu.bounds().size.height - 4.0
        )
    );
    assert_eq!(
        menu.item_bounds(0).unwrap().origin,
        menu.content_bounds().origin
    );
    assert_eq!(
        scene.rects()[1].shadow(),
        Some(
            BoxShadow::new(Color::rgba(0, 0, 0, 24))
                .with_offset(Point::new(0.0, 1.0))
                .with_blur_radius(10.0)
        )
    );
    assert_eq!(
        scene.rects()[2].shadow(),
        Some(
            BoxShadow::new(Color::rgba(0, 0, 0, 36))
                .with_offset(Point::new(0.0, 4.0))
                .with_blur_radius(6.0)
        )
    );
    assert_eq!(scene.rects()[2].corner_radii(), CornerRadii::uniform(4.0));
}

#[test]
fn defaults_to_the_first_enabled_item() {
    let menu = context_menu(vec![
        ContextMenuItem::new("Disabled", ButtonState::Disabled),
        ContextMenuItem::new("First enabled", ButtonState::Resting),
    ]);
    let mut scene = UiScene::new(Color::TRANSPARENT);

    menu.paint(&mut scene);

    assert_eq!(menu.selected_index(), Some(1));
    assert_eq!(scene.rects()[4].fill(), SELECTED);
}

#[test]
fn explicit_selection_drives_paint_and_hit_geometry() {
    let menu = context_menu(vec![
        ContextMenuItem::new("Pin", ButtonState::Resting),
        ContextMenuItem::new("Close", ButtonState::Focused),
    ])
    .with_selection(ContextMenuSelection::Item(1));
    let selected_bounds = menu.item_bounds(1).unwrap();
    let mut scene = UiScene::new(Color::TRANSPARENT);

    menu.paint(&mut scene);

    assert_eq!(menu.selected_index(), Some(1));
    assert_eq!(scene.rects()[4].fill(), SELECTED);
    assert_eq!(
        menu.hit_test(Point::new(
            selected_bounds.origin.x + 1.0,
            selected_bounds.origin.y + 1.0
        )),
        Some(1)
    );
}
