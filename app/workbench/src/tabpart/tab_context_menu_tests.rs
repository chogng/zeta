use super::TAB_CONTEXT_MENU;
use super::TabContextMenu;
use super::TabContextMenuAction;
use super::TabContextMenuState;
use super::TabContextMenuStyle;
use crate::TabInputKey;
use crate::tabpart::identity::WINDOW;
use zeta_protocol::SessionId;
use zui::ui::AccessibilityRole;
use zui::ui::Color;
use zui::ui::ElementId;
use zui::ui::InteractionFrame;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::UiDispatch;
use zui::ui::UiFrame;

fn session_tab() -> TabInputKey {
    TabInputKey::session(SessionId::new("session-1").unwrap())
}

fn style() -> TabContextMenuStyle {
    TabContextMenuStyle::new(
        Color::WHITE,
        Color::rgb(220, 220, 220),
        Color::rgb(30, 30, 30),
        Color::rgb(235, 235, 237),
    )
}

#[test]
fn menu_owns_current_generic_tab_actions() {
    let mut state = TabContextMenuState::default();
    let target = session_tab();
    state.open_unpinned(target.clone(), Point::new(80.0, 120.0), None);
    let dispatch = UiDispatch::default();
    let menu = TabContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &state,
        style(),
        WINDOW,
        &dispatch,
    )
    .unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&menu);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);

    assert_eq!(state.target_tab(), Some(&target));
    assert_eq!(
        TabContextMenuAction::ALL.map(|action| action.label(false)),
        ["Pin", "Close", "Move to new group"]
    );
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == TAB_CONTEXT_MENU)
            .unwrap()
            .role,
        AccessibilityRole::Menu
    );
}

#[test]
fn pinned_target_changes_the_first_action_to_unpin() {
    let mut state = TabContextMenuState::default();
    state.open_pinned(
        session_tab(),
        Point::new(20.0, 30.0),
        Some(ElementId::from_raw(99)),
    );
    let dispatch = UiDispatch::default();
    let menu = TabContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 500.0, 400.0),
        &state,
        style(),
        WINDOW,
        &dispatch,
    )
    .unwrap();

    assert!(state.target_is_pinned());
    assert_eq!(TabContextMenuAction::TogglePin.label(true), "Unpin");
    assert!(menu.item_bounds(0).is_some());
}
