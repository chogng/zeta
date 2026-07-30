use super::{InteractionEffect, PointerFeedback, ShellHitMap, ShellInteraction, ShellTarget};
use zeta_ui::{Point, Rect};

#[test]
fn titlebar_press_requests_native_window_drag() {
    let mut hit_map = ShellHitMap::default();
    hit_map.register(
        Rect::from_xywh(0.0, 0.0, 200.0, 35.0),
        ShellTarget::WindowDrag,
    );
    let mut interaction = ShellInteraction::default();

    interaction.pointer_moved(Point::new(100.0, 20.0), &hit_map);

    assert_eq!(
        interaction.press_primary(),
        InteractionEffect::StartWindowDrag
    );
    assert_eq!(interaction.pointer_feedback(), PointerFeedback::Default);
}

#[test]
fn terminal_surface_uses_text_pointer_feedback() {
    let hit_map = ShellHitMap::default();
    let mut interaction = ShellInteraction::default();

    interaction.pointer_moved(Point::new(100.0, 100.0), &hit_map);

    assert_eq!(interaction.press_primary(), InteractionEffect::None);
    assert_eq!(interaction.release_primary(), InteractionEffect::None);
    assert_eq!(interaction.pointer_feedback(), PointerFeedback::Text);
}

#[test]
fn leaving_a_registered_region_requests_one_redraw() {
    let mut hit_map = ShellHitMap::default();
    hit_map.register(
        Rect::from_xywh(0.0, 0.0, 200.0, 35.0),
        ShellTarget::WindowDrag,
    );
    let mut interaction = ShellInteraction::default();
    interaction.pointer_moved(Point::new(100.0, 20.0), &hit_map);

    assert_eq!(interaction.pointer_left(), InteractionEffect::Redraw);
    assert_eq!(interaction.pointer_left(), InteractionEffect::None);
}
