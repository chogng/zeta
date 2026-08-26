use super::WorkbenchTab;
use super::WorkbenchTabList;
use super::tab_input_element_id;
use crate::shell_interaction::{
    ACTIVE_SESSION_TAB, SESSION_TAB_LIST, SETTINGS_WORKBENCH_TAB, session_tab_id,
};
use crate::shell_style::SHELL_PALETTE;
use crate::tab_input::TabInputKey;
use crate::tab_input::TabInputModel;
use zeta_icons::icons;
use zeta_protocol::{Session, SessionId, SessionStatus};
use zeta_ui::{Color, Component, CornerRadii, FontWeight, Point, Rect, UiScene};
use zui::ui::{
    AccessibilityRole, AccessibilitySelection, InteractionFrame, UiDispatch, UiFrame, UiIntent,
};

fn second_session_tab() -> zui::ui::ElementId {
    session_tab_id(1)
}

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

#[test]
fn selected_tab_input_resolves_to_a_ui_element_only_at_the_presentation_boundary() {
    let first = session("session-1", "First terminal");
    let second = session("session-2", "Second terminal");
    let first_key = TabInputKey::session(first.session_id.clone());
    let second_key = TabInputKey::session(second.session_id.clone());
    let mut model = TabInputModel::default();

    model.upsert_session(&first, "~/first");
    model.upsert_session(&second, "~/second");

    assert_eq!(
        tab_input_element_id(model.inputs(), Some(&first_key)),
        ACTIVE_SESSION_TAB
    );
    assert_eq!(
        tab_input_element_id(model.inputs(), Some(&second_key)),
        session_tab_id(1)
    );
    assert_eq!(
        tab_input_element_id(model.inputs(), Some(&TabInputKey::Settings),),
        SETTINGS_WORKBENCH_TAB
    );
    assert_eq!(
        tab_input_element_id(model.inputs(), None),
        ACTIVE_SESSION_TAB
    );
}

#[test]
fn session_tabs_render_status_and_two_line_information_with_selected_semantics() {
    let dispatch = UiDispatch::default();
    let tabs = [
        WorkbenchTab::new(ACTIVE_SESSION_TAB, "zeterm", "~/Desktop/zeta", "Thinking"),
        WorkbenchTab::new(
            second_session_tab(),
            "Review terminal navigation",
            "~/Desktop/another-workspace-with-a-long-name",
            "Planning",
        ),
    ];
    let list = WorkbenchTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        &tabs,
        ACTIVE_SESSION_TAB,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&list);
    let scene = frame.scene();

    assert_eq!(
        scene
            .text_blocks()
            .iter()
            .map(|text| text.text())
            .collect::<Vec<_>>(),
        [
            "zeterm",
            "~/Desktop/zeta",
            "Review terminal navigation",
            "~/Desktop/another-workspace-with-a-long-name"
        ]
    );
    let first_bounds = list.tab_list().tab_bounds(0).unwrap();
    let second_bounds = list.tab_list().tab_bounds(1).unwrap();
    assert_eq!(second_bounds.origin.y - first_bounds.bottom(), 6.0);
    assert_eq!(
        frame.interaction().target_at(Point::new(
            first_bounds.origin.x + 4.0,
            first_bounds.origin.y + 4.0
        )),
        Some(ACTIVE_SESSION_TAB)
    );
    let nodes = frame.interaction().accessibility_nodes(&dispatch);
    let tab = nodes
        .iter()
        .find(|node| node.id == ACTIVE_SESSION_TAB)
        .unwrap();
    assert_eq!(tab.parent, Some(SESSION_TAB_LIST));
    assert_eq!(tab.role, AccessibilityRole::Tab);
    assert_eq!(tab.selection, AccessibilitySelection::Selected);
    assert!(tab.focusable);
    let second_tab = nodes
        .iter()
        .find(|node| node.id == second_session_tab())
        .unwrap();
    assert_eq!(second_tab.selection, AccessibilitySelection::Unselected);
    assert_eq!(
        second_tab.label,
        "Review terminal navigation, ~/Desktop/another-workspace-with-a-long-name, Planning"
    );

    let selected_background = scene.rects()[0];
    assert_eq!(
        selected_background.fill(),
        SHELL_PALETTE.session_tab_highlight
    );
    assert_eq!(selected_background.border().widths().left, 0.0);
    assert_eq!(
        selected_background.corner_radii(),
        CornerRadii::uniform(4.0)
    );
    assert_eq!(scene.rects()[1].fill(), Color::TRANSPARENT);
    assert_eq!(scene.rects()[2].bounds().size.height, 36.0);
    assert_eq!(scene.rects()[2].fill(), Color::WHITE);
    assert_eq!(scene.rects()[2].corner_radii(), CornerRadii::uniform(18.0));

    let name = &scene.text_blocks()[0];
    let workspace = &scene.text_blocks()[1];
    assert_eq!(name.style().weight(), FontWeight::Bold);
    assert_eq!(name.style().color(), SHELL_PALETTE.text);
    assert_eq!(name.bounds().width, workspace.bounds().width);
    assert!(name.origin().x + name.bounds().width <= first_bounds.right());
}

#[test]
fn hovering_an_unselected_tab_uses_the_same_light_gray_highlight() {
    let mut dispatch = UiDispatch::default();
    let tabs = [
        WorkbenchTab::new(ACTIVE_SESSION_TAB, "zeterm", "~/Desktop/zeta", "Active"),
        WorkbenchTab::new(second_session_tab(), "Second", "~/Desktop/second", "Active"),
    ];
    let resting = WorkbenchTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        &tabs,
        ACTIVE_SESSION_TAB,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&resting);
    let second_bounds = resting.tab_list().tab_bounds(1).unwrap();
    dispatch.pointer_moved(
        Point::new(second_bounds.origin.x + 2.0, second_bounds.origin.y + 2.0),
        frame.interaction(),
    );
    let hovered = WorkbenchTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        &tabs,
        ACTIVE_SESSION_TAB,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut scene = UiScene::new(SHELL_PALETTE.background);

    hovered.paint(&mut scene);

    assert_eq!(scene.rects()[1].fill(), SHELL_PALETTE.session_tab_highlight);
}

#[test]
fn clicking_an_unselected_tab_emits_its_stable_activation_intent() {
    let mut dispatch = UiDispatch::default();
    let tabs = [
        WorkbenchTab::new(ACTIVE_SESSION_TAB, "zeterm", "~/Desktop/zeta", "Active"),
        WorkbenchTab::new(second_session_tab(), "Second", "~/Desktop/second", "Active"),
    ];
    let list = WorkbenchTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        &tabs,
        ACTIVE_SESSION_TAB,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&list);
    let second_bounds = list.tab_list().tab_bounds(1).unwrap();
    let point = Point::new(second_bounds.origin.x + 2.0, second_bounds.origin.y + 2.0);

    dispatch.pointer_moved(point, frame.interaction());
    dispatch.press_primary(frame.interaction());
    let outcome = dispatch.release_primary(point, frame.interaction());

    assert_eq!(
        outcome.intent,
        Some(UiIntent::Activate(second_session_tab()))
    );
}

#[test]
fn settings_workbench_tab_renders_as_a_selectable_gear_item() {
    let mut dispatch = UiDispatch::default();
    let tabs = [WorkbenchTab::settings(SETTINGS_WORKBENCH_TAB)];
    let list = WorkbenchTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        &tabs,
        SETTINGS_WORKBENCH_TAB,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&list);

    assert_eq!(frame.scene().icons()[0].icon(), icons::GEAR);
    assert_eq!(frame.scene().rects()[1].fill(), SHELL_PALETTE.surface);
    assert_eq!(
        frame.scene().rects()[1].corner_radii(),
        CornerRadii::uniform(18.0)
    );
    let node = frame
        .interaction()
        .accessibility_nodes(&dispatch)
        .into_iter()
        .find(|node| node.id == SETTINGS_WORKBENCH_TAB)
        .expect("settings workbench tab should be registered");
    assert_eq!(node.label, "Settings");
    assert_eq!(node.selection, AccessibilitySelection::Selected);

    let bounds = list.tab_list().tab_bounds(0).expect("settings tab bounds");
    let point = Point::new(bounds.origin.x + 2.0, bounds.origin.y + 2.0);
    dispatch.pointer_moved(point, frame.interaction());
    dispatch.press_primary(frame.interaction());
    let outcome = dispatch.release_primary(point, frame.interaction());
    assert_eq!(
        outcome.intent,
        Some(UiIntent::Activate(SETTINGS_WORKBENCH_TAB))
    );
}
