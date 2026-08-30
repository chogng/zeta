use super::Titlebar;
use super::TitlebarInsets;
use crate::Color;
use crate::Point;
use crate::Rect;
use crate::TabPart;
use crate::tabpart::identity::{
    CHANGES_PANE_BUTTON, TAB_CONTAINER_TOGGLE, TITLEBAR, TITLEBAR_SETTINGS_BUTTON,
    TITLEBAR_SETTINGS_CLOSE, TITLEBAR_TAB_CONTAINER,
};
use crate::tabpart::test_style;
use crate::{TabInputKey, TabIntent, tab_intent_for_element};
use zeta_icons::icons;
use zui::ui::InteractionFrame;
use zui::ui::UiDispatch;
use zui::ui::UiFrame;
use zui::ui::UiIntent;

#[test]
fn collapsed_tab_part_hides_tabs_and_keeps_titlebar_actions() {
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
        TitlebarInsets::new(70.0, 110.0),
        &dispatch,
    );
    frame.draw_component(&titlebar);

    assert_eq!(frame.scene().icons().len(), 3);
    assert_eq!(
        frame.scene().icons()[0].icon(),
        icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY
    );
    assert_eq!(frame.scene().icons()[1].icon(), icons::DIFF);
    assert_eq!(frame.scene().icons()[2].icon(), icons::GEAR);
    assert!(frame.interaction().node(TITLEBAR_SETTINGS_CLOSE).is_none());
    assert!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .all(|text| text.text() != "Settings")
    );
    assert!(frame.interaction().node(TITLEBAR_TAB_CONTAINER).is_none());

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

    let settings_bounds = frame
        .interaction()
        .node(TITLEBAR_SETTINGS_BUTTON)
        .expect("collapsed Tab Part keeps Settings available")
        .bounds();
    let settings_point = Point::new(
        settings_bounds.origin.x + settings_bounds.size.width * 0.5,
        settings_bounds.origin.y + settings_bounds.size.height * 0.5,
    );
    dispatch.pointer_moved(settings_point, frame.interaction());
    dispatch.press_primary(frame.interaction());
    assert_eq!(
        dispatch
            .release_primary(settings_point, frame.interaction())
            .intent,
        Some(UiIntent::Activate(TITLEBAR_SETTINGS_BUTTON))
    );

    let changes_bounds = frame
        .interaction()
        .node(CHANGES_PANE_BUTTON)
        .expect("changes action")
        .bounds();
    let changes_point = Point::new(
        changes_bounds.origin.x + changes_bounds.size.width * 0.5,
        changes_bounds.origin.y + changes_bounds.size.height * 0.5,
    );
    dispatch.pointer_moved(changes_point, frame.interaction());
    dispatch.press_primary(frame.interaction());
    assert!(dispatch.is_pressed(CHANGES_PANE_BUTTON));
    assert_eq!(
        dispatch
            .release_primary(changes_point, frame.interaction())
            .intent,
        Some(UiIntent::Activate(CHANGES_PANE_BUTTON))
    );
}

#[test]
fn expanded_tab_part_uses_the_changes_icon_without_titlebar_tabs() {
    let part = TabPart::default();
    let dispatch = UiDispatch::default();
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 32.0),
        test_style(),
        &part,
        part.active_tab_key(),
        true,
        TitlebarInsets::NONE,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);

    frame.draw_component(&titlebar);

    assert_eq!(frame.scene().icons().len(), 3);
    assert_eq!(frame.scene().icons()[0].icon(), icons::LAYOUT_SIDEBAR_LEFT);
    assert_eq!(frame.scene().icons()[1].icon(), icons::DIFF);
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
