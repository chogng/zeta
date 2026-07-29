use super::{LogicalViewport, ShellLayout, build_shell_presentation};
use crate::shell_interaction::ShellInteraction;
use zeta_ui::Point;

#[test]
fn shell_layout_places_product_titlebar_above_sidebar_and_main() {
    let layout = ShellLayout::for_viewport(LogicalViewport {
        width: 1000.0,
        height: 700.0,
    })
    .unwrap();

    assert_eq!(layout.titlebar.origin.y, 0.0);
    assert_eq!(layout.titlebar.size.height, 35.0);
    assert_eq!(layout.titlebar.bottom(), layout.sidebar.origin.y);
    assert_eq!(layout.sidebar.right(), layout.main.origin.x);
    assert_eq!(layout.sidebar.origin.y, layout.main.origin.y);
    assert_eq!(layout.sidebar.size.height, layout.main.size.height);
    assert!(layout.composer.origin.x >= layout.main.origin.x);
    assert!(layout.composer.right() <= layout.main.right());
    assert!(layout.transcript.bottom() < layout.composer.origin.y);
}

#[test]
fn shell_presentation_contains_structural_paint_and_interactive_regions() {
    let mut interaction = ShellInteraction::default();
    let viewport = LogicalViewport {
        width: 1000.0,
        height: 700.0,
    };
    let presentation = build_shell_presentation(viewport, &interaction);

    assert!(presentation.scene.rects().len() >= 10);
    assert_eq!(presentation.scene.icons().len(), 1);
    assert!(presentation.scene.text_blocks().len() >= 12);
    assert_eq!(
        interaction.pointer_moved(Point::new(930.0, 24.0), &presentation.hit_map),
        crate::shell_interaction::InteractionEffect::Redraw
    );
}

#[test]
fn theme_button_click_changes_scene_background() {
    let mut interaction = ShellInteraction::default();
    let viewport = LogicalViewport {
        width: 1000.0,
        height: 700.0,
    };
    let initial = build_shell_presentation(viewport, &interaction);
    let initial_background = initial.scene.background();

    interaction.pointer_moved(Point::new(930.0, 24.0), &initial.hit_map);
    interaction.press_primary();
    interaction.release_primary();
    let toggled = build_shell_presentation(viewport, &interaction);

    assert_ne!(toggled.scene.background(), initial_background);
}

#[test]
fn compact_viewport_uses_bounded_fallback_scene() {
    let presentation = build_shell_presentation(
        LogicalViewport {
            width: 220.0,
            height: 180.0,
        },
        &ShellInteraction::default(),
    );

    assert_eq!(presentation.scene.rects().len(), 1);
    assert_eq!(presentation.scene.text_blocks().len(), 1);
}
