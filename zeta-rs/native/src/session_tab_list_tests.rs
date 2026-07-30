use super::{SessionTab, SessionTabList};
use crate::shell_interaction::{ACTIVE_SESSION_TAB, SESSION_TAB_LIST};
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{Color, Component, CornerRadii, FontWeight, Point, Rect, UiScene};
use zeta_ui_dispatch::{
    AccessibilityRole, AccessibilitySelection, ElementId, InteractionFrame, UiDispatch,
};

const SECOND_SESSION_TAB: ElementId = ElementId::scoped(1, 16);

#[test]
fn session_tabs_render_status_and_two_line_information_with_selected_semantics() {
    let dispatch = UiDispatch::default();
    let tabs = [
        SessionTab::new(ACTIVE_SESSION_TAB, "zeterm", "~/Desktop/zeta", "Thinking"),
        SessionTab::new(
            SECOND_SESSION_TAB,
            "Review terminal navigation",
            "~/Desktop/another-workspace-with-a-long-name",
            "Planning",
        ),
    ];
    let list = SessionTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        &tabs,
        ACTIVE_SESSION_TAB,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut scene = UiScene::new(SHELL_PALETTE.background);
    let mut frame = InteractionFrame::default();

    list.paint(&mut scene);
    list.register_interactions(&mut frame);

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
        frame.target_at(Point::new(
            first_bounds.origin.x + 4.0,
            first_bounds.origin.y + 4.0
        )),
        Some(ACTIVE_SESSION_TAB)
    );
    let nodes = frame.accessibility_nodes(&dispatch);
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
        .find(|node| node.id == SECOND_SESSION_TAB)
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
        SessionTab::new(ACTIVE_SESSION_TAB, "zeterm", "~/Desktop/zeta", "Active"),
        SessionTab::new(SECOND_SESSION_TAB, "Second", "~/Desktop/second", "Active"),
    ];
    let resting = SessionTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        &tabs,
        ACTIVE_SESSION_TAB,
        SHELL_PALETTE,
        &dispatch,
    );
    let mut frame = InteractionFrame::default();
    resting.register_interactions(&mut frame);
    let second_bounds = resting.tab_list().tab_bounds(1).unwrap();
    dispatch.pointer_moved(
        Point::new(second_bounds.origin.x + 2.0, second_bounds.origin.y + 2.0),
        &frame,
    );
    let hovered = SessionTabList::new(
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
