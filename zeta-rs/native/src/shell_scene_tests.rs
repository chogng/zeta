use super::{LogicalViewport, ShellLayout, build_shell_presentation};
use crate::shell_interaction::ShellInteraction;
use zeta_ui::{CaretVisibility, Point, TextInputLayoutEngine};

const CARET_VISIBLE: CaretVisibility = CaretVisibility::Visible;

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
    let mut text_layout = TextInputLayoutEngine::new();
    let presentation =
        build_shell_presentation(viewport, &interaction, &mut text_layout, CARET_VISIBLE);

    assert!(presentation.scene.rects().len() >= 9);
    assert!(presentation.scene.icons().is_empty());
    assert!(presentation.scene.text_blocks().len() >= 11);
    assert_eq!(
        interaction.pointer_moved(Point::new(930.0, 24.0), &presentation.hit_map),
        crate::shell_interaction::InteractionEffect::Redraw
    );
}

#[test]
fn compact_viewport_uses_bounded_fallback_scene() {
    let mut text_layout = TextInputLayoutEngine::new();
    let presentation = build_shell_presentation(
        LogicalViewport {
            width: 220.0,
            height: 180.0,
        },
        &ShellInteraction::default(),
        &mut text_layout,
        CARET_VISIBLE,
    );

    assert_eq!(presentation.scene.rects().len(), 1);
    assert_eq!(presentation.scene.text_blocks().len(), 1);
}

#[test]
fn focused_composer_exposes_shaped_ime_cursor_area() {
    let viewport = LogicalViewport {
        width: 1000.0,
        height: 700.0,
    };
    let mut interaction = ShellInteraction::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let initial = build_shell_presentation(viewport, &interaction, &mut text_layout, CARET_VISIBLE);

    interaction.pointer_moved(Point::new(500.0, 650.0), &initial.hit_map);
    interaction.press_primary();
    interaction.release_primary();
    let focused = build_shell_presentation(viewport, &interaction, &mut text_layout, CARET_VISIBLE);

    let caret = focused.ime_cursor_area.unwrap();
    assert!(caret.origin.x > 250.0);
    assert!(caret.origin.y > 600.0);
    assert!(caret.size.height > 0.0);
}
