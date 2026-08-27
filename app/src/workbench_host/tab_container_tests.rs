//! Workbench Tab Container projection tests.

use super::TabContainer;
use super::TabContainerPlacement;
use super::tab_input_element_id;
use crate::shell_interaction::FIRST_TAB_CONTAINER_SESSION_TAB;
use crate::shell_interaction::FIRST_TITLEBAR_SESSION_TAB;
use crate::shell_interaction::TAB_CONTAINER_SETTINGS_TAB;
use crate::shell_interaction::TITLEBAR_SETTINGS_TAB;
use crate::shell_interaction::session_tab_id;
use crate::shell_interaction::tab_group_list_id;
use crate::shell_interaction::titlebar_session_tab_id;
use crate::shell_style::SHELL_PALETTE;
use crate::workbench_host::TabGroupId;
use crate::workbench_host::TabInputKey;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_ui::Color;
use zeta_ui::FontWeight;
use zeta_ui::Point;
use zeta_ui::Rect;
use crate::workbench_host::TabPart;
use zui::ui::AccessibilityRole;
use zui::ui::AccessibilitySelection;
use zui::ui::InteractionFrame;
use zui::ui::UiDispatch;
use zui::ui::UiFrame;
use zui::ui::UiIntent;

fn session(id: &str, title: &str) -> Session {
    Session {
        session_id: SessionId::new(id).unwrap(),
        title: title.to_owned(),
        status: SessionStatus::Active,
        model: None,
        workspace: None,
        sequence: 1,
        threads: Vec::new(),
    }
}

fn part_with_two_sessions() -> (TabPart, TabInputKey, TabInputKey) {
    let first = session("session-1", "First terminal");
    let second = session("session-2", "Second terminal");
    let first_key = TabInputKey::session(first.session_id.clone());
    let second_key = TabInputKey::session(second.session_id.clone());
    let mut part = TabPart::default();
    part.upsert_session(&first, "~/first");
    part.upsert_session(&second, "~/second");
    (part, first_key, second_key)
}

#[test]
fn each_mount_resolves_distinct_ui_identity_for_the_same_tab_input() {
    let (part, first_key, second_key) = part_with_two_sessions();

    assert_eq!(
        tab_input_element_id(&part, Some(&first_key), TabContainerPlacement::Body),
        FIRST_TAB_CONTAINER_SESSION_TAB
    );
    assert_eq!(
        tab_input_element_id(&part, Some(&second_key), TabContainerPlacement::Body),
        session_tab_id(1)
    );
    assert_eq!(
        tab_input_element_id(&part, Some(&first_key), TabContainerPlacement::Titlebar),
        FIRST_TITLEBAR_SESSION_TAB
    );
    assert_eq!(
        tab_input_element_id(
            &part,
            Some(&TabInputKey::Settings),
            TabContainerPlacement::Titlebar,
        ),
        TITLEBAR_SETTINGS_TAB
    );
}

#[test]
fn body_mount_arranges_tabs_vertically_with_two_line_session_information() {
    let (part, first_key, _) = part_with_two_sessions();
    let dispatch = UiDispatch::default();
    let bounds = Rect::from_xywh(0.0, 36.0, 220.0, 664.0);
    let container = TabContainer::from_tab_part(
        bounds,
        bounds,
        &part,
        Some(&first_key),
        TabContainerPlacement::Body,
        SHELL_PALETTE,
        &dispatch,
    );
    let layouts = container.group_layouts();
    let first_bounds = layouts[0].tab_list.tab_bounds(0).unwrap();
    let second_bounds = layouts[0].tab_list.tab_bounds(1).unwrap();
    assert_eq!(second_bounds.origin.y - first_bounds.bottom(), 6.0);
    drop(layouts);
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);

    frame.draw_component(&container);

    assert_eq!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .map(|text| text.text())
            .collect::<Vec<_>>(),
        [
            "First terminal",
            "~/first",
            "Second terminal",
            "~/second",
            "Settings",
            "Application",
        ]
    );
    let selected = frame
        .interaction()
        .accessibility_nodes(&dispatch)
        .into_iter()
        .find(|node| node.id == FIRST_TAB_CONTAINER_SESSION_TAB)
        .unwrap();
    assert_eq!(selected.role, AccessibilityRole::Tab);
    assert_eq!(selected.selection, AccessibilitySelection::Selected);
    assert_eq!(
        selected.parent,
        Some(tab_group_list_id(TabGroupId::DEFAULT))
    );
    assert_eq!(
        frame.scene().text_blocks()[0].style().weight(),
        FontWeight::Bold
    );
}

#[test]
fn titlebar_mount_arranges_tabs_horizontally_and_emits_activation() {
    let (part, first_key, _) = part_with_two_sessions();
    let mut dispatch = UiDispatch::default();
    let bounds = Rect::from_xywh(40.0, 0.0, 700.0, 32.0);
    let container = TabContainer::from_tab_part(
        bounds,
        bounds,
        &part,
        Some(&first_key),
        TabContainerPlacement::Titlebar,
        SHELL_PALETTE,
        &dispatch,
    );
    let layouts = container.group_layouts();
    let first_bounds = layouts[0].tab_list.tab_bounds(0).unwrap();
    let second_bounds = layouts[0].tab_list.tab_bounds(1).unwrap();
    assert_eq!(second_bounds.origin.x - first_bounds.right(), 4.0);
    assert_eq!(first_bounds.size.height, 24.0);
    drop(layouts);
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&container);
    let second_bounds = container.group_layouts()[0].tab_list.tab_bounds(1).unwrap();
    let point = Point::new(second_bounds.origin.x + 2.0, second_bounds.origin.y + 2.0);

    dispatch.pointer_moved(point, frame.interaction());
    dispatch.press_primary(frame.interaction());
    let outcome = dispatch.release_primary(point, frame.interaction());

    assert_eq!(
        outcome.intent,
        Some(UiIntent::Activate(titlebar_session_tab_id(1)))
    );
    assert!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .all(|text| text.text() != "~/first")
    );
}

#[test]
fn browser_style_groups_project_as_separate_tab_lists_with_group_labels() {
    let (mut part, first_key, second_key) = part_with_two_sessions();
    let group = part
        .group_tabs([first_key, second_key], "Terminal work")
        .unwrap();
    let dispatch = UiDispatch::default();
    let bounds = Rect::from_xywh(0.0, 0.0, 240.0, 700.0);
    let container = TabContainer::from_tab_part(
        bounds,
        bounds,
        &part,
        part.active_tab_key(),
        TabContainerPlacement::Body,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);

    frame.draw_component(&container);

    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    assert!(nodes.iter().any(|node| node.id == tab_group_list_id(group)));
    assert_eq!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .filter(|text| text.text() == "Terminal work")
            .count(),
        1
    );
    assert_eq!(
        nodes
            .iter()
            .find(|node| node.id == FIRST_TAB_CONTAINER_SESSION_TAB)
            .unwrap()
            .parent,
        Some(tab_group_list_id(group))
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.id == TAB_CONTAINER_SETTINGS_TAB)
    );
}
