//! Workbench tabs tests.

use super::TabContainer;
use super::TabContainerPlacement;
use super::mounted_tab_element_id;
use super::tab_input_element_id;
use super::tab_intent_for_element;
use crate::Color;
use crate::FontWeight;
use crate::Point;
use crate::Rect;
use crate::TabGroupId;
use crate::TabInput;
use crate::TabInputKey;
use crate::TabInputMetadata;
use crate::TabIntent;
use crate::TabPart;
use crate::TabStatus;
use crate::tabpart::identity::{
    FIRST_TAB_CONTAINER_SESSION_TAB, FIRST_TITLEBAR_SESSION_TAB, TAB_CONTAINER_SETTINGS_ACTION,
    TAB_CONTAINER_SETTINGS_CLOSE, TAB_CONTAINER_SETTINGS_TAB, TITLEBAR_SETTINGS_ACTION,
    TITLEBAR_SETTINGS_CLOSE, TITLEBAR_SETTINGS_TAB, session_tab_action_id, session_tab_close_id,
    session_tab_id, tab_group_list_id, titlebar_session_tab_action_id,
    titlebar_session_tab_close_id, titlebar_session_tab_id,
};
use crate::tabpart::test_style;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
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
        next_approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
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
    part.upsert_session_input(TabInput::session(
        first.session_id,
        TabInputMetadata::new(first.title, "~/first").with_status(TabStatus::busy("Active")),
    ));
    part.upsert_session_input(TabInput::session(
        second.session_id,
        TabInputMetadata::new(second.title, "~/second").with_status(TabStatus::busy("Active")),
    ));
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
        session_tab_id(part.tab_id(&second_key).unwrap())
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
    assert_eq!(
        mounted_tab_element_id(
            &part,
            &TabInputKey::session(SessionId::new("missing-session").unwrap()),
            TabContainerPlacement::Body,
        ),
        None
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
        test_style(),
        &dispatch,
    );
    let layouts = container.group_layouts();
    let first_bounds = layouts[0].tab_list.tab_bounds(0).unwrap();
    let second_bounds = layouts[0].tab_list.tab_bounds(1).unwrap();
    assert_eq!(second_bounds.origin.y - first_bounds.bottom(), 6.0);
    drop(layouts);
    let mut frame = UiFrame::<InteractionFrame>::new(Color::WHITE);

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
    assert_eq!(
        frame.scene().text_blocks()[1].style().color(),
        Color::rgb(38, 38, 41)
    );
}

#[test]
fn titlebar_mount_arranges_tabs_horizontally_and_emits_activation() {
    let (part, first_key, second_key) = part_with_two_sessions();
    let mut dispatch = UiDispatch::default();
    let bounds = Rect::from_xywh(40.0, 0.0, 700.0, 32.0);
    let container = TabContainer::from_tab_part(
        bounds,
        bounds,
        &part,
        Some(&first_key),
        TabContainerPlacement::Titlebar,
        test_style(),
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
        Some(UiIntent::Activate(titlebar_session_tab_id(
            part.tab_id(&second_key).unwrap()
        )))
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
fn close_button_is_a_child_action_and_resolves_to_the_stable_tab_key() {
    let (part, first_key, _) = part_with_two_sessions();
    let mut dispatch = UiDispatch::default();
    let bounds = Rect::from_xywh(40.0, 0.0, 700.0, 32.0);
    let container = TabContainer::from_tab_part(
        bounds,
        bounds,
        &part,
        Some(&first_key),
        TabContainerPlacement::Titlebar,
        test_style(),
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&container);
    let tab_bounds = frame
        .interaction()
        .node(titlebar_session_tab_id(part.tab_id(&first_key).unwrap()))
        .unwrap()
        .bounds();
    drop(container);
    dispatch.pointer_moved(
        Point::new(tab_bounds.origin.x + 2.0, tab_bounds.origin.y + 2.0),
        frame.interaction(),
    );
    drop(frame);
    let container = TabContainer::from_tab_part(
        bounds,
        bounds,
        &part,
        Some(&first_key),
        TabContainerPlacement::Titlebar,
        test_style(),
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    frame.draw_component(&container);
    let tab_id = part.tab_id(&first_key).unwrap();
    let close_id = titlebar_session_tab_close_id(tab_id);
    let close_node = frame
        .interaction()
        .accessibility_nodes(&dispatch)
        .into_iter()
        .find(|node| node.id == close_id)
        .unwrap();
    let point = Point::new(
        close_node.bounds.origin.x + 2.0,
        close_node.bounds.origin.y + 2.0,
    );

    dispatch.pointer_moved(point, frame.interaction());
    dispatch.press_primary(frame.interaction());
    let outcome = dispatch.release_primary(point, frame.interaction());

    assert_eq!(close_node.role, AccessibilityRole::Button);
    assert_eq!(close_node.parent, Some(titlebar_session_tab_id(tab_id)));
    assert_eq!(outcome.intent, Some(UiIntent::Activate(close_id)));
    assert_eq!(
        tab_intent_for_element(&part, close_id),
        Some(TabIntent::Close(first_key))
    );
}

#[test]
fn tab_actions_button_is_hover_only_and_opens_actions_for_the_stable_tab_key() {
    let (part, first_key, second_key) = part_with_two_sessions();
    let tab_id = part.tab_id(&first_key).unwrap();
    let mounts = [
        (
            TabContainerPlacement::Body,
            Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
            session_tab_id(tab_id),
            session_tab_action_id(tab_id),
            session_tab_close_id(tab_id),
            TAB_CONTAINER_SETTINGS_ACTION,
        ),
        (
            TabContainerPlacement::Titlebar,
            Rect::from_xywh(40.0, 0.0, 700.0, 32.0),
            titlebar_session_tab_id(tab_id),
            titlebar_session_tab_action_id(tab_id),
            titlebar_session_tab_close_id(tab_id),
            TITLEBAR_SETTINGS_ACTION,
        ),
    ];

    for (placement, bounds, tab_element, action_element, close_element, settings_action) in mounts {
        let mut dispatch = UiDispatch::default();
        let container = TabContainer::from_tab_part(
            bounds,
            bounds,
            &part,
            Some(&second_key),
            placement,
            test_style(),
            &dispatch,
        );
        let mut initial = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
        initial.draw_component(&container);
        assert!(initial.interaction().node(action_element).is_none());
        assert!(initial.interaction().node(settings_action).is_none());
        let tab_bounds = initial.interaction().node(tab_element).unwrap().bounds();
        drop(container);

        dispatch.pointer_moved(
            Point::new(tab_bounds.origin.x + 2.0, tab_bounds.origin.y + 2.0),
            initial.interaction(),
        );
        let container = TabContainer::from_tab_part(
            bounds,
            bounds,
            &part,
            Some(&second_key),
            placement,
            test_style(),
            &dispatch,
        );
        let mut hovered = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
        hovered.draw_component(&container);

        assert_eq!(
            hovered.interaction().node(action_element).unwrap().role(),
            AccessibilityRole::Button
        );
        assert_eq!(
            tab_intent_for_element(&part, action_element),
            Some(TabIntent::OpenActions(first_key.clone()))
        );
        assert_eq!(
            tab_intent_for_element(&part, settings_action),
            Some(TabIntent::OpenActions(TabInputKey::Settings))
        );
        assert_eq!(
            hovered
                .scene()
                .icons()
                .iter()
                .filter(|icon| icon.icon() == zeta_icons::icons::ELLIPSIS)
                .count(),
            1
        );
        let tab_background = hovered
            .scene()
            .rects()
            .iter()
            .find(|rect| rect.bounds() == tab_bounds)
            .expect("hovered tab background");
        assert_eq!(tab_background.fill(), Color::rgb(226, 226, 228));
        let action_bounds = hovered
            .interaction()
            .node(action_element)
            .expect("visible tab action")
            .bounds();
        let close_bounds = hovered
            .interaction()
            .node(close_element)
            .expect("visible tab close")
            .bounds();
        let action_bar_bounds = Rect::from_xywh(
            action_bounds.origin.x,
            action_bounds.origin.y,
            close_bounds.right() - action_bounds.origin.x,
            action_bounds.size.height,
        );
        let action_background = hovered
            .scene()
            .rects()
            .iter()
            .find(|rect| rect.bounds() == action_bar_bounds)
            .expect("tab action background");
        assert_eq!(action_background.fill(), Color::rgb(245, 245, 246));
        assert_eq!(
            hovered
                .scene()
                .icons()
                .iter()
                .find(|icon| icon.icon() == zeta_icons::icons::ELLIPSIS)
                .expect("tab actions icon")
                .color(),
            Color::rgb(126, 126, 132)
        );
    }
}

#[test]
fn explicitly_visible_action_bar_remains_rendered_without_hover() {
    let (part, first_key, second_key) = part_with_two_sessions();
    let tab_id = part.tab_id(&first_key).unwrap();
    let mounts = [
        (
            TabContainerPlacement::Body,
            Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
            session_tab_id(tab_id),
            session_tab_action_id(tab_id),
            session_tab_close_id(tab_id),
        ),
        (
            TabContainerPlacement::Titlebar,
            Rect::from_xywh(40.0, 0.0, 700.0, 32.0),
            titlebar_session_tab_id(tab_id),
            titlebar_session_tab_action_id(tab_id),
            titlebar_session_tab_close_id(tab_id),
        ),
    ];

    for (placement, bounds, tab_element, action_element, close_element) in mounts {
        let dispatch = UiDispatch::default();
        let container = TabContainer::from_tab_part(
            bounds,
            bounds,
            &part,
            Some(&second_key),
            placement,
            test_style(),
            &dispatch,
        )
        .with_visible_action_bar(tab_element);
        let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);

        frame.draw_component(&container);

        assert!(frame.interaction().node(action_element).is_some());
        assert!(frame.interaction().node(close_element).is_some());
        assert!(!dispatch.is_hovered(tab_element));
    }
}

#[test]
fn every_hovered_tab_has_an_action_bar_in_each_mount() {
    let (part, first_key, second_key) = part_with_two_sessions();
    let first_id = part.tab_id(&first_key).unwrap();
    let second_id = part.tab_id(&second_key).unwrap();
    let mounts = [
        (
            TabContainerPlacement::Body,
            Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
            [
                session_tab_close_id(first_id),
                session_tab_close_id(second_id),
                TAB_CONTAINER_SETTINGS_CLOSE,
            ],
        ),
        (
            TabContainerPlacement::Titlebar,
            Rect::from_xywh(40.0, 0.0, 700.0, 32.0),
            [
                titlebar_session_tab_close_id(first_id),
                titlebar_session_tab_close_id(second_id),
                TITLEBAR_SETTINGS_CLOSE,
            ],
        ),
    ];

    for (placement, bounds, close_ids) in mounts {
        let tab_ids = match placement {
            TabContainerPlacement::Body => [
                session_tab_id(first_id),
                session_tab_id(second_id),
                TAB_CONTAINER_SETTINGS_TAB,
            ],
            TabContainerPlacement::Titlebar => [
                titlebar_session_tab_id(first_id),
                titlebar_session_tab_id(second_id),
                TITLEBAR_SETTINGS_TAB,
            ],
        };
        let expected = [
            TabIntent::Close(first_key.clone()),
            TabIntent::Close(second_key.clone()),
            TabIntent::Close(TabInputKey::Settings),
        ];
        for ((tab_id, close_id), expected) in tab_ids.into_iter().zip(close_ids).zip(expected) {
            let mut dispatch = UiDispatch::default();
            let container = TabContainer::from_tab_part(
                bounds,
                bounds,
                &part,
                Some(&first_key),
                placement,
                test_style(),
                &dispatch,
            );
            let mut initial = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
            initial.draw_component(&container);
            let tab_bounds = initial.interaction().node(tab_id).unwrap().bounds();
            drop(container);
            dispatch.pointer_moved(
                Point::new(tab_bounds.origin.x + 2.0, tab_bounds.origin.y + 2.0),
                initial.interaction(),
            );
            drop(initial);
            let container = TabContainer::from_tab_part(
                bounds,
                bounds,
                &part,
                Some(&first_key),
                placement,
                test_style(),
                &dispatch,
            );
            let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
            frame.draw_component(&container);
            let node = frame.interaction().node(close_id).unwrap();
            assert_eq!(node.role(), AccessibilityRole::Button);
            assert_eq!(tab_intent_for_element(&part, close_id), Some(expected));
            assert_eq!(
                frame
                    .scene()
                    .icons()
                    .iter()
                    .filter(|icon| icon.icon() == zeta_icons::icons::CLOSE)
                    .count(),
                1
            );
        }
    }
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
        test_style(),
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
