use super::SessionTabList;
use crate::shell_interaction::{ACTIVE_SESSION_TAB, SESSION_TAB_LIST};
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{Component, Point, Rect, UiScene};
use zeta_ui_dispatch::{AccessibilityRole, AccessibilitySelection, InteractionFrame, UiDispatch};

#[test]
fn current_real_session_is_painted_and_registered_as_the_selected_tab() {
    let dispatch = UiDispatch::default();
    let list = SessionTabList::new(
        Rect::from_xywh(0.0, 36.0, 220.0, 664.0),
        "zeterm",
        "~/Desktop/zeta",
        "main",
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
        ["SESSIONS", "zeterm", "~/Desktop/zeta", "git:(main)"]
    );
    assert_eq!(
        frame.target_at(Point::new(
            list.tab_bounds().origin.x + 4.0,
            list.tab_bounds().origin.y + 4.0
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
}
