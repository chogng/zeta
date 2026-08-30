use super::{ContextMenu, ContextMenuStyle};
use crate::{
    ButtonBackgrounds, ButtonState, ButtonStyle, Color, ElementId, MenuIds, MenuItem,
    MenuSelection, MenuStyle, PaintRect, Point, Rect, Size, TextStyle, UiDispatch, UiFrame,
    UiScene,
};
use zui::ui::InteractionFrame;

const PARENT: ElementId = ElementId::scoped(92, 1);
const ROOT: ElementId = ElementId::scoped(92, 2);
const FIRST: ElementId = ElementId::scoped(92, 3);
const SECOND: ElementId = ElementId::scoped(92, 4);
const SURFACE: Color = Color::rgb(240, 241, 242);
const HEADER: Color = Color::rgb(210, 211, 212);

fn style() -> MenuStyle {
    MenuStyle::new(
        SURFACE,
        ButtonStyle::new(
            ButtonBackgrounds::new(Color::TRANSPARENT),
            TextStyle::new(13.0, Color::rgb(0, 0, 0)),
        ),
        Size::new(120.0, 28.0),
    )
}

#[test]
fn anchors_menu_content_and_keeps_its_interaction_tree() {
    let menu = ContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 400.0, 300.0),
        Rect::from_xywh(30.0, 20.0, 1.0, 1.0),
        "Tab actions",
        vec![
            MenuItem::new(FIRST, "Pin", ButtonState::Resting),
            MenuItem::new(SECOND, "Close", ButtonState::Resting),
        ],
        MenuIds::new(PARENT, ROOT),
        ContextMenuStyle::new(style()),
    )
    .with_selection(MenuSelection::None);
    let dispatch = UiDispatch::default();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);

    frame.draw_component(&menu);

    assert_eq!(menu.menu_root(), ROOT);
    assert_eq!(menu.bounds().origin, Point::new(30.0, 21.0));
    assert_eq!(menu.content_bounds().origin, Point::new(32.0, 23.0));
    assert_eq!(
        menu.item_bounds(0).unwrap().origin,
        menu.content_bounds().origin
    );
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert!(nodes.iter().any(|node| node.id == ROOT));
    assert!(nodes.iter().any(|node| node.id == FIRST));
    assert!(nodes.iter().any(|node| node.id == SECOND));
}

#[test]
fn header_content_is_composed_inside_the_anchored_menu() {
    let menu = ContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 400.0, 300.0),
        Rect::from_xywh(30.0, 260.0, 80.0, 24.0),
        "Branches",
        vec![MenuItem::new(FIRST, "main", ButtonState::Resting)],
        MenuIds::new(PARENT, ROOT),
        ContextMenuStyle::new(style().with_header_height(36.0)),
    );
    let header_bounds = menu.header_bounds().unwrap();
    let mut scene = UiScene::new(Color::TRANSPARENT);

    menu.paint_with_header(&mut scene, |scene, bounds| {
        scene.draw_rect(PaintRect::new(bounds, HEADER));
    });

    assert_eq!(header_bounds.origin, menu.content_bounds().origin);
    assert_eq!(
        menu.item_bounds(0).unwrap().origin.y,
        header_bounds.bottom()
    );
    assert_eq!(menu.bounds().size.height, 68.0);
    assert!(scene.rects().iter().any(|rect| rect.fill() == HEADER));
}
