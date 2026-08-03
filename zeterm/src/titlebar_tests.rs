use super::Titlebar;
use crate::agent_sidebar::AgentSidebarState;
use crate::session_sidebar::SessionSidebarState;
use crate::shell_interaction::{
    AGENT_SIDEBAR_TOGGLE, LANGUAGE_SERVER_SETTINGS_TOGGLE, SESSION_SIDEBAR_TOGGLE, TITLEBAR,
};
use crate::shell_style::SHELL_PALETTE;
use zeta_icons::icons;
use zeta_ui::{Color, Component, Point, Rect, UiScene};
use zeta_winit::WindowControlInsets;
use zui::{InteractionFrame, UiDispatch, UiFrame, UiIntent};

#[test]
fn titlebar_places_actions_after_native_window_controls_and_component_gap() {
    let mut frame = UiFrame::<InteractionFrame>::new(Color::TRANSPARENT);
    let mut dispatch = UiDispatch::default();
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 32.0),
        SHELL_PALETTE,
        SessionSidebarState::default(),
        AgentSidebarState::default(),
        WindowControlInsets::from_logical_sides(70.0, 110.0),
        &dispatch,
    );
    frame.draw_component(&titlebar);
    let scene = frame.scene();

    assert_eq!(scene.icons().len(), 3);
    assert!(scene.text_blocks().is_empty());
    assert_eq!(
        scene.icons()[0].icon(),
        icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY
    );
    assert_eq!(scene.icons()[1].icon(), icons::SETTINGS);
    assert_eq!(
        scene.icons()[2].icon(),
        icons::LAYOUT_SIDEBAR_RIGHT_OFF_EMPTY
    );
    assert_eq!(
        scene.icons()[0].bounds(),
        Rect::from_xywh(81.0, 7.0, 18.0, 18.0)
    );
    assert_eq!(
        scene.icons()[1].bounds(),
        Rect::from_xywh(829.0, 7.0, 18.0, 18.0)
    );
    assert_eq!(
        scene.icons()[2].bounds(),
        Rect::from_xywh(861.0, 7.0, 18.0, 18.0)
    );
    dispatch.pointer_moved(Point::new(400.0, 16.0), frame.interaction());
    assert_eq!(
        dispatch.press_primary(frame.interaction()).intent,
        Some(UiIntent::StartWindowDrag(TITLEBAR))
    );

    dispatch.pointer_moved(Point::new(83.0, 16.0), frame.interaction());
    assert_eq!(
        dispatch.press_primary(frame.interaction()).invalidation,
        zui::DispatchInvalidation::Paint
    );
    assert!(dispatch.is_pressed(SESSION_SIDEBAR_TOGGLE));
    assert_eq!(
        dispatch
            .release_primary(Point::new(83.0, 16.0), frame.interaction())
            .intent,
        Some(UiIntent::Activate(SESSION_SIDEBAR_TOGGLE))
    );

    dispatch.pointer_moved(Point::new(837.0, 16.0), frame.interaction());
    let _ = dispatch.press_primary(frame.interaction());
    assert!(dispatch.is_pressed(LANGUAGE_SERVER_SETTINGS_TOGGLE));
    assert_eq!(
        dispatch
            .release_primary(Point::new(837.0, 16.0), frame.interaction())
            .intent,
        Some(UiIntent::Activate(LANGUAGE_SERVER_SETTINGS_TOGGLE))
    );

    dispatch.pointer_moved(Point::new(859.0, 16.0), frame.interaction());
    assert_eq!(
        dispatch.press_primary(frame.interaction()).invalidation,
        zui::DispatchInvalidation::Paint
    );
    assert!(dispatch.is_pressed(AGENT_SIDEBAR_TOGGLE));
    assert_eq!(
        dispatch
            .release_primary(Point::new(859.0, 16.0), frame.interaction())
            .intent,
        Some(UiIntent::Activate(AGENT_SIDEBAR_TOGGLE))
    );
}

#[test]
fn expanded_sidebars_use_the_left_and_right_icons() {
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 32.0),
        SHELL_PALETTE,
        SessionSidebarState::expanded(),
        AgentSidebarState::expanded(),
        WindowControlInsets::NONE,
        &UiDispatch::default(),
    );
    let mut scene = UiScene::new(Color::TRANSPARENT);

    titlebar.paint(&mut scene);

    assert_eq!(scene.icons()[0].icon(), icons::LAYOUT_SIDEBAR_LEFT);
    assert_eq!(scene.icons()[1].icon(), icons::SETTINGS);
    assert_eq!(scene.icons()[2].icon(), icons::LAYOUT_SIDEBAR_RIGHT);
}
