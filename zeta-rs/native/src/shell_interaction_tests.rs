use super::{InteractionEffect, SessionId, ShellHitMap, ShellInteraction, ShellTarget};
use zeta_ui::{
    Point, Rect, TextInputCommand, TextInputCompositionCursor, TextInputCompositionEvent,
};

#[test]
fn last_registered_hit_region_has_priority() {
    let mut hit_map = ShellHitMap::default();
    hit_map.register(
        Rect::from_xywh(0.0, 0.0, 100.0, 40.0),
        ShellTarget::WindowDrag,
    );
    hit_map.register(
        Rect::from_xywh(60.0, 5.0, 35.0, 30.0),
        ShellTarget::Session(SessionId::Renderer),
    );
    let mut interaction = ShellInteraction::default();

    interaction.pointer_moved(Point::new(70.0, 20.0), &hit_map);
    assert_eq!(interaction.press_primary(), InteractionEffect::Redraw);
}

#[test]
fn session_and_composer_clicks_update_owned_state() {
    let mut hit_map = ShellHitMap::default();
    hit_map.register(
        Rect::from_xywh(0.0, 0.0, 100.0, 40.0),
        ShellTarget::Session(SessionId::Renderer),
    );
    hit_map.register(
        Rect::from_xywh(0.0, 50.0, 100.0, 40.0),
        ShellTarget::Composer,
    );
    let mut interaction = ShellInteraction::default();

    interaction.pointer_moved(Point::new(10.0, 20.0), &hit_map);
    interaction.press_primary();
    interaction.release_primary();
    assert_eq!(interaction.selected_session(), SessionId::Renderer);

    interaction.pointer_moved(Point::new(10.0, 60.0), &hit_map);
    interaction.press_primary();
    interaction.release_primary();
    assert!(interaction.composer_focused());
}

#[test]
fn titlebar_press_requests_native_window_drag() {
    let mut hit_map = ShellHitMap::default();
    hit_map.register(
        Rect::from_xywh(0.0, 0.0, 200.0, 50.0),
        ShellTarget::WindowDrag,
    );
    let mut interaction = ShellInteraction::default();

    interaction.pointer_moved(Point::new(100.0, 20.0), &hit_map);
    assert_eq!(
        interaction.press_primary(),
        InteractionEffect::StartWindowDrag
    );
}

#[test]
fn composer_focus_gates_editing_and_clears_preedit_on_blur() {
    let mut hit_map = ShellHitMap::default();
    hit_map.register(
        Rect::from_xywh(0.0, 0.0, 120.0, 40.0),
        ShellTarget::Composer,
    );
    let mut interaction = ShellInteraction::default();

    assert_eq!(
        interaction.edit_composer(TextInputCommand::Insert("ignored".to_owned())),
        InteractionEffect::None
    );
    interaction.pointer_moved(Point::new(10.0, 10.0), &hit_map);
    interaction.press_primary();
    assert_eq!(
        interaction.release_primary(),
        InteractionEffect::FocusComposer
    );
    interaction.edit_composer(TextInputCommand::Insert("hello".to_owned()));
    interaction.update_composition(TextInputCompositionEvent::Preedit {
        text: "世".to_owned(),
        cursor: TextInputCompositionCursor::Visible(3..3),
    });

    assert_eq!(interaction.composer().text(), "hello");
    assert!(interaction.composer().composition().is_some());
    assert_eq!(
        interaction.window_focus_lost(),
        InteractionEffect::BlurComposer
    );
    assert!(interaction.composer().composition().is_none());
}
