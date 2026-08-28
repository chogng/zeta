use super::Titlebar;
use super::TitlebarInsets;
use crate::Color;
use crate::PaneInputKind;
use crate::Point;
use crate::Rect;
use crate::TabPart;
use crate::tabpart::identity::{
    TAB_CONTAINER_TOGGLE, TITLEBAR, TITLEBAR_SETTINGS_BUTTON, TITLEBAR_SETTINGS_CLOSE,
    TITLEBAR_SETTINGS_TAB, TITLEBAR_TAB_CONTAINER, WORKSPACE_PANE_TOGGLE,
};
use crate::tabpart::test_style;
use crate::{TabInputKey, TabIntent, tab_intent_for_element};
use zeta_icons::icons;
use zui::ui::InteractionFrame;
use zui::ui::UiDispatch;
use zui::ui::UiFrame;
use zui::ui::UiIntent;

#[test]
fn titlebar_mounts_tabs_between_window_controls_and_actions() {
    let mut part = TabPart::default();
    part.activate_settings();
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    let mut dispatch = UiDispatch::default();
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 32.0),
        test_style(),
        &part,
        part.active_tab_key(),
        false,
        None,
        TitlebarInsets::new(70.0, 110.0),
        &dispatch,
    );
    frame.draw_component(&titlebar);

    assert_eq!(frame.scene().icons().len(), 3);
    assert_eq!(frame.scene().icons()[0].icon(), icons::GEAR);
    assert_eq!(
        frame.scene().icons()[1].icon(),
        icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY
    );
    assert_eq!(
        frame.scene().icons()[2].icon(),
        icons::LAYOUT_SIDEBAR_RIGHT_OFF_EMPTY
    );
    assert!(frame.interaction().node(TITLEBAR_SETTINGS_CLOSE).is_none());
    assert!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .any(|text| text.text() == "Settings")
    );

    dispatch.pointer_moved(Point::new(400.0, 16.0), frame.interaction());
    assert_eq!(
        dispatch.press_primary(frame.interaction()).intent,
        Some(UiIntent::StartWindowDrag(TITLEBAR))
    );

    dispatch.pointer_moved(Point::new(83.0, 16.0), frame.interaction());
    dispatch.press_primary(frame.interaction());
    assert!(dispatch.is_pressed(TAB_CONTAINER_TOGGLE));
    assert_eq!(
        dispatch
            .release_primary(Point::new(83.0, 16.0), frame.interaction())
            .intent,
        Some(UiIntent::Activate(TAB_CONTAINER_TOGGLE))
    );

    dispatch.pointer_moved(Point::new(120.0, 16.0), frame.interaction());
    dispatch.press_primary(frame.interaction());
    assert_eq!(
        dispatch
            .release_primary(Point::new(120.0, 16.0), frame.interaction())
            .intent,
        Some(UiIntent::Activate(TITLEBAR_SETTINGS_TAB))
    );

    dispatch.pointer_moved(Point::new(867.0, 16.0), frame.interaction());
    dispatch.press_primary(frame.interaction());
    assert!(dispatch.is_pressed(WORKSPACE_PANE_TOGGLE));
    assert_eq!(
        dispatch
            .release_primary(Point::new(867.0, 16.0), frame.interaction())
            .intent,
        Some(UiIntent::Activate(WORKSPACE_PANE_TOGGLE))
    );
}

#[test]
fn expanded_tab_container_omits_horizontal_tabs_and_uses_the_active_icons() {
    let part = TabPart::default();
    let dispatch = UiDispatch::default();
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 32.0),
        test_style(),
        &part,
        part.active_tab_key(),
        true,
        Some(PaneInputKind::Files),
        TitlebarInsets::NONE,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);

    frame.draw_component(&titlebar);

    assert_eq!(frame.scene().icons().len(), 3);
    assert_eq!(frame.scene().icons()[0].icon(), icons::LAYOUT_SIDEBAR_LEFT);
    assert_eq!(frame.scene().icons()[1].icon(), icons::LAYOUT_SIDEBAR_RIGHT);
    assert_eq!(frame.scene().icons()[2].icon(), icons::GEAR);
    assert!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .all(|text| text.text() != "Settings")
    );
    assert!(frame.interaction().node(TITLEBAR_TAB_CONTAINER).is_none());
    assert_eq!(
        frame
            .interaction()
            .node(TITLEBAR_SETTINGS_BUTTON)
            .expect("expanded tabs expose the titlebar Settings button")
            .role(),
        zui::ui::AccessibilityRole::Button
    );
    assert_eq!(
        tab_intent_for_element(&part, TITLEBAR_SETTINGS_BUTTON),
        Some(TabIntent::Activate(TabInputKey::Settings))
    );
}
