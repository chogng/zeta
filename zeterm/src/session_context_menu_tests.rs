use super::{SessionContextMenu, SessionContextMenuState, update_session_context_menu_pointer};
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, COMPOSER, SESSION_CONTEXT_MENU, SessionContextMenuAction,
};
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{Color, Edges, Point, Rect};
use zui::ui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, FocusBehavior, InteractionFrame,
    UiDispatch, UiFrame, UiNode,
};

#[test]
fn context_menu_places_four_vertical_actions_beside_the_pointer() {
    let mut state = SessionContextMenuState::default();
    state.open(ACTIVE_SESSION_TAB, Point::new(80.0, 120.0), Some(COMPOSER));
    let dispatch = UiDispatch::default();
    let menu = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        state,
        SHELL_PALETTE,
        &dispatch,
    )
    .unwrap();

    assert_eq!(state.target_session(), Some(ACTIVE_SESSION_TAB));
    assert_eq!(menu.bounds().origin, Point::new(80.0, 123.0));
    assert_eq!(menu.bounds().size, zeta_ui::Size::new(164.0, 124.0));
    assert_eq!(
        menu.item_bounds(0).unwrap().origin,
        Point::new(menu.bounds().origin.x + 2.0, menu.bounds().origin.y + 2.0)
    );
    assert_eq!(menu.selected_index(), Some(0));
    for index in 1..SessionContextMenuAction::ALL.len() {
        assert_eq!(
            menu.item_bounds(index).unwrap().origin.y
                - menu.item_bounds(index - 1).unwrap().bottom(),
            0.0
        );
    }
}

#[test]
fn context_menu_paints_in_an_overlay_and_registers_menu_semantics() {
    let mut state = SessionContextMenuState::default();
    state.open(ACTIVE_SESSION_TAB, Point::new(80.0, 120.0), Some(COMPOSER));
    let mut dispatch = UiDispatch::default();
    let resting = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        state,
        SHELL_PALETTE,
        &dispatch,
    )
    .unwrap();
    let second_bounds = resting.item_bounds(1).unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&resting);
    dispatch.pointer_moved(
        Point::new(second_bounds.origin.x + 2.0, second_bounds.origin.y + 2.0),
        frame.interaction(),
    );
    let hovered = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        state,
        SHELL_PALETTE,
        &dispatch,
    )
    .unwrap();
    let mut hovered_frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    hovered_frame.draw_component(&hovered);
    let scene = hovered_frame.scene();

    assert_eq!(scene.rects()[2].bounds(), hovered.bounds());
    assert_eq!(scene.rects()[2].border().widths(), Edges::uniform(0.0));
    assert_eq!(scene.rects()[3].fill(), Color::TRANSPARENT);
    assert_eq!(scene.rects()[4].fill(), SHELL_PALETTE.session_tab_highlight);
    assert_eq!(hovered.selected_index(), Some(1));
    let nodes = hovered_frame.interaction().accessibility_nodes(&dispatch);
    let menu_node = nodes
        .iter()
        .find(|node| node.id == SESSION_CONTEXT_MENU)
        .unwrap();
    assert_eq!(menu_node.role, AccessibilityRole::Menu);
    for action in SessionContextMenuAction::ALL {
        let item = nodes
            .iter()
            .find(|node| node.id == action.element_id())
            .unwrap();
        assert_eq!(item.parent, Some(SESSION_CONTEXT_MENU));
        assert_eq!(item.role, AccessibilityRole::MenuItem);
        assert_eq!(item.label, action.label());
        assert_eq!(
            item.selection,
            if action == SessionContextMenuAction::Close {
                AccessibilitySelection::Selected
            } else {
                AccessibilitySelection::Unselected
            }
        );
    }
    assert_eq!(
        dispatch.pointer_feedback(hovered_frame.interaction()),
        CursorFeedback::Pointer
    );
}

#[test]
fn dismiss_returns_the_focus_identity_captured_when_opening() {
    let mut state = SessionContextMenuState::default();
    state.open(ACTIVE_SESSION_TAB, Point::new(10.0, 20.0), Some(COMPOSER));

    assert!(state.is_open());
    assert_eq!(state.dismiss(), Some(COMPOSER));
    assert!(!state.is_open());
}

#[test]
fn pointer_hover_moves_menu_focus_and_stays_on_the_last_item_after_exit() {
    let mut state = SessionContextMenuState::default();
    state.open(ACTIVE_SESSION_TAB, Point::new(80.0, 120.0), Some(COMPOSER));
    let mut dispatch = UiDispatch::default();
    let menu = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        state,
        SHELL_PALETTE,
        &dispatch,
    )
    .unwrap();
    let third_bounds = menu.item_bounds(2).unwrap();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.interaction_mut().register(
        UiNode::new(
            COMPOSER,
            Rect::from_xywh(400.0, 600.0, 400.0, 40.0),
            AccessibilityRole::TextInput,
            "Command input",
        )
        .with_focus(FocusBehavior::TabStop),
    );
    frame.draw_component(&menu);
    dispatch.reconcile_focus(
        frame.interaction(),
        SessionContextMenuAction::Pin.element_id(),
    );

    update_session_context_menu_pointer(
        &mut dispatch,
        Point::new(third_bounds.origin.x + 2.0, third_bounds.origin.y + 2.0),
        frame.interaction(),
    );
    update_session_context_menu_pointer(
        &mut dispatch,
        Point::new(500.0, 620.0),
        frame.interaction(),
    );

    assert!(dispatch.is_focused(SessionContextMenuAction::Rename.element_id()));
    assert!(!dispatch.is_hovered(COMPOSER));
    let exited = SessionContextMenu::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        state,
        SHELL_PALETTE,
        &dispatch,
    )
    .unwrap();
    assert_eq!(exited.selected_index(), Some(2));
}
