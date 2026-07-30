use super::Titlebar;
use crate::shell_interaction::{SIDEBAR_TOGGLE, SessionSidebarState, TITLEBAR};
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{Color, Component, Point, Rect, UiScene};
use zeta_ui_dispatch::{InteractionFrame, UiDispatch, UiIntent};
use zeta_winit::WindowControlInsets;

#[test]
fn titlebar_places_actions_after_native_window_controls_and_component_gap() {
    let mut frame = InteractionFrame::default();
    let mut dispatch = UiDispatch::default();
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 32.0),
        SHELL_PALETTE,
        SessionSidebarState::Collapsed,
        WindowControlInsets::from_logical_sides(70.0, 0.0),
        &dispatch,
    );
    titlebar.register_interactions(&mut frame);
    let mut scene = UiScene::new(Color::TRANSPARENT);
    titlebar.paint(&mut scene);

    assert_eq!(scene.icons().len(), 1);
    assert!(scene.text_blocks().is_empty());
    assert_eq!(
        scene.icons()[0].bounds(),
        Rect::from_xywh(83.0, 7.0, 18.0, 18.0)
    );
    dispatch.pointer_moved(Point::new(400.0, 16.0), &frame);
    assert_eq!(
        dispatch.press_primary(&frame).intent,
        Some(UiIntent::StartWindowDrag(TITLEBAR))
    );

    dispatch.pointer_moved(Point::new(83.0, 16.0), &frame);
    assert_eq!(
        dispatch.press_primary(&frame).invalidation,
        zeta_ui_dispatch::DispatchInvalidation::Paint
    );
    assert!(dispatch.is_pressed(SIDEBAR_TOGGLE));
    assert_eq!(
        dispatch
            .release_primary(Point::new(83.0, 16.0), &frame)
            .intent,
        Some(UiIntent::Activate(SIDEBAR_TOGGLE))
    );
}
