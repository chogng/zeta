use super::{Menu, MenuIds, MenuItem, MenuSelection, MenuStyle};
use crate::{
    AccessibilitySelection, BoxShadow, ButtonBackgrounds, ButtonState, ButtonStyle, Color,
    CornerRadii, ElementId, Point, Rect, Size, TextStyle, UiDispatch, UiFrame, UiScene,
};
use zui::ui::InteractionFrame;

const PARENT: ElementId = ElementId::scoped(91, 1);
const ROOT: ElementId = ElementId::scoped(91, 2);
const FIRST: ElementId = ElementId::scoped(91, 3);
const SECOND: ElementId = ElementId::scoped(91, 4);
const SURFACE: Color = Color::rgb(240, 241, 242);
const SELECTED: Color = Color::rgb(210, 211, 212);

fn style() -> MenuStyle {
    MenuStyle::new(
        SURFACE,
        ButtonStyle::new(
            ButtonBackgrounds::new(Color::TRANSPARENT),
            TextStyle::new(13.0, Color::rgb(0, 0, 0)),
        )
        .with_selected_backgrounds(ButtonBackgrounds::new(SELECTED)),
        Size::new(120.0, 28.0),
    )
}

fn menu(items: Vec<MenuItem>) -> Menu {
    Menu::new(
        Rect::from_xywh(30.0, 20.0, 124.0, 60.0),
        "Actions",
        items,
        MenuIds::new(PARENT, ROOT),
        style(),
    )
}

#[test]
fn owns_native_menu_shadow_padding_and_corner_radius() {
    let menu = menu(vec![
        MenuItem::new(FIRST, "Pin", ButtonState::Resting),
        MenuItem::new(SECOND, "Close", ButtonState::Resting),
    ]);
    let mut scene = UiScene::new(Color::TRANSPARENT);

    scene.draw_component(&menu);

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
        scene.rects()[0].shadow(),
        Some(
            BoxShadow::new(Color::rgba(0, 0, 0, 64))
                .with_offset(Point::new(0.0, 4.0))
                .with_blur_radius(26.352_942)
        )
    );
    assert_eq!(scene.rects()[0].corner_radii(), CornerRadii::uniform(4.0));
}

#[test]
fn selection_skips_disabled_items_and_drives_hit_geometry() {
    let menu = menu(vec![
        MenuItem::new(FIRST, "Disabled", ButtonState::Disabled),
        MenuItem::new(SECOND, "First enabled", ButtonState::Focused),
    ]);
    let selected_bounds = menu.item_bounds(1).unwrap();
    let mut scene = UiScene::new(Color::TRANSPARENT);

    scene.draw_component(&menu);

    assert_eq!(menu.selected_index(), Some(1));
    assert_eq!(scene.rects()[2].fill(), SELECTED);
    assert_eq!(
        menu.hit_test(Point::new(
            selected_bounds.origin.x + 1.0,
            selected_bounds.origin.y + 1.0
        )),
        Some(1)
    );
}

#[test]
fn explicit_selection_is_reflected_in_the_accessibility_tree() {
    let menu = menu(vec![
        MenuItem::new(FIRST, "Pin", ButtonState::Resting),
        MenuItem::new(SECOND, "Close", ButtonState::Focused),
    ])
    .with_selection(MenuSelection::Item(1));
    let dispatch = UiDispatch::default();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);

    frame.draw_component(&menu);

    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    let root = nodes.iter().find(|node| node.id == ROOT).unwrap();
    let first = nodes.iter().find(|node| node.id == FIRST).unwrap();
    let second = nodes.iter().find(|node| node.id == SECOND).unwrap();
    assert_eq!(root.label, "Actions");
    assert_eq!(root.parent, Some(PARENT));
    assert_eq!(first.parent, Some(ROOT));
    assert_eq!(second.parent, Some(ROOT));
    assert_eq!(first.selection, AccessibilitySelection::Unselected);
    assert_eq!(second.selection, AccessibilitySelection::Selected);
}

#[test]
fn reserved_header_shifts_items_and_stays_inside_the_menu_tree() {
    let menu = Menu::new(
        Rect::from_xywh(30.0, 20.0, 124.0, 68.0),
        "Branches",
        vec![MenuItem::new(FIRST, "main", ButtonState::Resting)],
        MenuIds::new(PARENT, ROOT),
        style().with_header_height(36.0),
    );
    let header_bounds = menu.header_bounds().unwrap();
    let mut scene = UiScene::new(Color::TRANSPARENT);

    menu.paint_with_header(&mut scene, |scene, bounds| {
        scene.draw_rect(crate::PaintRect::new(bounds, SELECTED));
    });

    assert_eq!(header_bounds.origin, menu.content_bounds().origin);
    assert_eq!(header_bounds.size.height, 36.0);
    assert_eq!(
        menu.item_bounds(0).unwrap().origin.y,
        header_bounds.bottom()
    );
    assert!(scene.rects().iter().any(|rect| rect.fill() == SELECTED));
}
