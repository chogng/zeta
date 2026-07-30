use super::{
    LogicalViewport, ShellLayout, ShellPresentation, build_shell_presentation,
    terminal_grid_size_for_viewport, terminal_mouse_position_for_viewport,
};
use crate::shell_interaction::{InteractionEffect, ShellInteraction, ShellTarget};
use crate::terminal_projection::scroll_limit;
use zeta_terminal::{GridSize, ScreenBuffer, TerminalCore};
use zeta_ui::{CaretVisibility, Point, TextInput, TextInputLayoutEngine};

fn viewport() -> LogicalViewport {
    LogicalViewport {
        width: 1000.0,
        height: 700.0,
    }
}

fn presentation(terminal: Option<&TerminalCore>, scroll_offset: usize) -> ShellPresentation {
    let composer = TextInput::new();
    let mut text_layout = TextInputLayoutEngine::new();
    build_shell_presentation(
        viewport(),
        terminal,
        scroll_offset,
        None,
        &composer,
        &mut text_layout,
        CaretVisibility::Visible,
    )
}

#[test]
fn primary_layout_keeps_output_above_a_bottom_composer() {
    let layout = ShellLayout::for_viewport(viewport()).unwrap();

    assert_eq!(layout.titlebar.origin.y, 0.0);
    assert_eq!(layout.titlebar.size.height, 35.0);
    assert_eq!(layout.main.origin.x, 0.0);
    assert_eq!(layout.main.bottom(), 700.0);
    assert!(layout.output.bottom() < layout.composer.origin.y);
    assert_eq!(layout.composer.bottom(), 676.0);
}

#[test]
fn primary_presentation_has_block_output_and_a_fixed_command_editor() {
    let presentation = presentation(None, 0);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(visible_text.contains(&"zeterm"));
    assert!(visible_text.contains(&"Starting shell…"));
    assert!(visible_text.contains(&"Enter a command…"));
    assert!(!visible_text.contains(&"SESSIONS"));
    assert!(presentation.scene.icons().is_empty());
}

#[test]
fn titlebar_drags_the_window_and_composer_is_a_registered_input_region() {
    let presentation = presentation(None, 0);
    let mut interaction = ShellInteraction::default();

    assert_eq!(
        interaction.pointer_moved(Point::new(500.0, 17.0), &presentation.hit_map),
        InteractionEffect::Redraw
    );
    assert_eq!(
        interaction.press_primary(),
        InteractionEffect::StartWindowDrag
    );
    assert_eq!(
        interaction.pointer_moved(Point::new(500.0, 640.0), &presentation.hit_map),
        InteractionEffect::Redraw
    );
    assert_eq!(interaction.press_primary(), InteractionEffect::None);
    assert_eq!(
        presentation.hit_map.target_at(Point::new(500.0, 640.0)),
        Some(ShellTarget::Composer)
    );
}

#[test]
fn compact_viewport_uses_bounded_fallback_scene() {
    let composer = TextInput::new();
    let mut text_layout = TextInputLayoutEngine::new();
    let presentation = build_shell_presentation(
        LogicalViewport {
            width: 220.0,
            height: 100.0,
        },
        None,
        0,
        None,
        &composer,
        &mut text_layout,
        CaretVisibility::Visible,
    );

    assert_eq!(presentation.scene.rects().len(), 1);
    assert_eq!(presentation.scene.text_blocks().len(), 1);
    assert_eq!(presentation.scene.text_blocks()[0].text(), "zeterm");
}

#[test]
fn primary_reserves_rows_for_composer_while_alternate_screen_uses_full_height() {
    let primary = terminal_grid_size_for_viewport(viewport(), ScreenBuffer::Primary);
    let alternate = terminal_grid_size_for_viewport(viewport(), ScreenBuffer::Alternate);

    assert_eq!(primary, GridSize::new(29, 119));
    assert_eq!(alternate, GridSize::new(34, 119));
}

#[test]
fn primary_pointer_coordinates_are_limited_to_the_output_region() {
    let first = terminal_mouse_position_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        Point::new(24.0, 59.0),
    )
    .unwrap();
    let composer = terminal_mouse_position_for_viewport(
        viewport(),
        ScreenBuffer::Primary,
        Point::new(40.0, 640.0),
    );

    assert_eq!((first.row(), first.col()), (0, 0));
    assert_eq!(composer, None);
}

#[test]
fn primary_block_list_is_projected_above_the_composer() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.process_output(b"$ ");
    terminal.start_command("printf hi");
    terminal.process_output(b"\x1b[32mhi\x1b[0m\r\n");

    let presentation = presentation(Some(&terminal), 0);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(visible_text.contains(&"❯ printf hi"));
    assert!(visible_text.contains(&"hi"));
    assert!(visible_text.contains(&"Enter a command…"));
}

#[test]
fn primary_block_transcript_can_project_an_older_viewport() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.start_command("history");
    for index in 0..80 {
        terminal.process_output(format!("line-{index}\r\n").as_bytes());
    }
    let capacity =
        terminal_grid_size_for_viewport(viewport(), ScreenBuffer::Primary).rows() as usize;
    let limit = scroll_limit(&terminal, capacity);

    let presentation = presentation(Some(&terminal), limit);
    let visible_text = presentation
        .scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();

    assert!(limit > 0);
    assert!(visible_text.contains(&"❯ history"));
    assert!(visible_text.contains(&"line-0"));
    assert!(!visible_text.contains(&"line-79"));
}

#[test]
fn primary_ime_candidate_position_comes_from_the_bottom_composer() {
    let terminal = TerminalCore::new(GridSize::new(29, 119));
    let layout = ShellLayout::for_viewport(viewport()).unwrap();

    let presentation = presentation(Some(&terminal), 0);
    let caret = presentation.ime_cursor_area.unwrap();

    assert!(layout.composer.contains(caret.origin));
}

#[test]
fn alternate_screen_ime_position_comes_from_the_terminal_cursor() {
    let mut terminal = TerminalCore::new(GridSize::new(34, 119));
    terminal.process_output(b"\x1b[?1049habc");

    let presentation = presentation(Some(&terminal), 0);
    let caret = presentation.ime_cursor_area.unwrap();

    assert_eq!(caret.origin, Point::new(48.0, 59.0));
    assert_eq!(caret.size.width, 8.0);
    assert_eq!(caret.size.height, 18.0);
}

#[test]
fn osc_title_is_projected_into_the_product_titlebar() {
    let mut terminal = TerminalCore::new(GridSize::new(29, 119));
    terminal.process_output(b"\x1b]2;project shell\x07");

    let presentation = presentation(Some(&terminal), 0);

    assert_eq!(presentation.scene.text_blocks()[0].text(), "project shell");
}
