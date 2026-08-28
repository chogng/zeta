use super::TAB_CONTEXT_MENU;
use super::TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP;
use super::TAB_RENAME_INPUT;
use super::TabContextMenu;
use super::TabContextMenuAction;
use super::TabContextMenuState;
use super::TabContextMenuStyle;
use super::tab_group_menu_element_id;
use crate::TabInputKey;
use crate::TabPart;
use crate::tabpart::identity::WINDOW;
use zeta_protocol::SessionId;
use zui::ui::AccessibilityRole;
use zui::ui::CaretVisibility;
use zui::ui::Color;
use zui::ui::ElementId;
use zui::ui::InteractionFrame;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::TextInputLayoutEngine;
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
    let part = TabPart::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let menu = TabContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &part,
        &state,
        CaretVisibility::Visible,
        style(),
        WINDOW,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&menu);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);

    assert_eq!(state.target_tab(), Some(&target));
    assert_eq!(
        TabContextMenuAction::ALL.map(|action| action.label(false)),
        ["Pin tab", "Close tab", "Move to group  ›", "Rename tab"]
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
    let part = TabPart::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let menu = TabContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 500.0, 400.0),
        &part,
        &state,
        CaretVisibility::Visible,
        style(),
        WINDOW,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();

    assert!(state.target_is_pinned());
    assert_eq!(TabContextMenuAction::TogglePin.label(true), "Unpin tab");
    assert!(menu.item_bounds(0).is_some());
}

#[test]
fn move_to_group_opens_a_secondary_menu_with_existing_and_new_group_targets() {
    let mut part = TabPart::default();
    let first = session_tab();
    let second = TabInputKey::session(SessionId::new("session-2").unwrap());
    part.upsert_session_input(crate::TabInput::session(
        first.session_id().unwrap().clone(),
        crate::TabInputMetadata::new("First", "~/first"),
    ));
    part.upsert_session_input(crate::TabInput::session(
        second.session_id().unwrap().clone(),
        crate::TabInputMetadata::new("Second", "~/second"),
    ));
    let group = part.group_tabs([second], "Other group").unwrap();
    let mut state = TabContextMenuState::default();
    state.open_unpinned(first, Point::new(80.0, 120.0), None);
    assert_eq!(
        state.activate(TabContextMenuAction::MoveToGroup.element_id()),
        super::TabContextMenuActivation::OpenGroupMenu
    );
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let menu = TabContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &part,
        &state,
        CaretVisibility::Visible,
        style(),
        WINDOW,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&menu);
    let nodes = frame.interaction().accessibility_nodes(&dispatch);

    assert!(
        nodes
            .iter()
            .any(|node| node.id == tab_group_menu_element_id(group))
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.id == TAB_CONTEXT_MENU_MOVE_TO_NEW_GROUP)
    );
}

#[test]
fn rename_action_presents_the_workbench_owned_text_input() {
    let mut part = TabPart::default();
    let target = session_tab();
    part.upsert_session_input(crate::TabInput::session(
        target.session_id().unwrap().clone(),
        crate::TabInputMetadata::new("First", "~/first"),
    ));
    let mut state = TabContextMenuState::default();
    state.open_unpinned(target, Point::new(80.0, 120.0), None);
    assert!(matches!(
        state.activate(TabContextMenuAction::Rename.element_id()),
        super::TabContextMenuActivation::BeginRename(_)
    ));
    assert!(state.set_rename_text("Renamed tab"));
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let menu = TabContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &part,
        &state,
        CaretVisibility::Visible,
        style(),
        WINDOW,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&menu);
    let rename = frame.interaction().node(TAB_RENAME_INPUT).unwrap();

    assert_eq!(rename.role(), AccessibilityRole::TextInput);
    assert_eq!(rename.value(), Some("Renamed tab"));
}
