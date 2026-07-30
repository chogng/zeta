use super::Titlebar;
use crate::shell_interaction::{InteractionEffect, ShellHitMap, ShellInteraction};
use crate::shell_style::SHELL_PALETTE;
use zeta_ui::{Color, Component, Point, Rect, UiScene};

#[test]
fn titlebar_paints_the_terminal_title_and_owns_window_drag() {
    let mut hit_map = ShellHitMap::default();
    let titlebar = Titlebar::new(
        Rect::from_xywh(0.0, 0.0, 1000.0, 35.0),
        "zeterm",
        SHELL_PALETTE,
    );
    titlebar.register_hit_regions(&mut hit_map);
    let mut scene = UiScene::new(Color::TRANSPARENT);
    titlebar.paint(&mut scene);
    let mut interaction = ShellInteraction::default();

    assert!(scene.icons().is_empty());
    assert_eq!(scene.text_blocks()[0].text(), "zeterm");
    interaction.pointer_moved(Point::new(400.0, 17.0), &hit_map);
    assert_eq!(
        interaction.press_primary(),
        InteractionEffect::StartWindowDrag
    );
}
