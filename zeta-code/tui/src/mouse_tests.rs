use super::ScreenSelection;
use super::ScreenSelectionOutcome;
use super::ScreenSelectionRange;
use super::text_in_range;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;

#[test]
fn drag_selection_normalizes_both_directions_and_remains_visible_after_release() {
    let mut selection = ScreenSelection::default();
    selection.begin(Position::new(8, 3));
    selection.drag(Position::new(2, 1));

    assert_eq!(
        selection.finish(Position::new(2, 1)),
        Some(ScreenSelectionOutcome::Copy(ScreenSelectionRange::new(
            Position::new(8, 3),
            Position::new(2, 1),
        )))
    );
    assert_eq!(
        selection.range(),
        Some(ScreenSelectionRange::new(
            Position::new(2, 1),
            Position::new(8, 3),
        ))
    );
}

#[test]
fn click_does_not_create_a_screen_selection() {
    let mut selection = ScreenSelection::default();
    let position = Position::new(4, 2);
    selection.begin(position);

    assert_eq!(
        selection.finish(position),
        Some(ScreenSelectionOutcome::Click(position))
    );
    assert_eq!(selection.range(), None);
}

#[test]
fn selected_screen_text_trims_row_padding_and_preserves_line_boundaries() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 3));
    buffer.set_string(0, 0, "alpha", Style::default());
    buffer.set_string(0, 1, "  beta", Style::default());
    buffer.set_string(0, 2, "gamma", Style::default());

    assert_eq!(
        text_in_range(
            &buffer,
            ScreenSelectionRange::new(Position::new(2, 0), Position::new(3, 2))
        ),
        Some("pha\n  beta\ngamm".into())
    );
}

#[test]
fn selected_screen_text_does_not_copy_wide_character_continuation_cells() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 1));
    buffer.set_string(0, 0, "你 ok", Style::default());

    assert_eq!(
        text_in_range(
            &buffer,
            ScreenSelectionRange::new(Position::new(0, 0), Position::new(4, 0))
        ),
        Some("你 ok".into())
    );
}

#[test]
fn selection_highlight_applies_to_every_rendered_surface_cell_in_the_range() {
    let mut selection = ScreenSelection::default();
    selection.begin(Position::new(1, 0));
    selection.drag(Position::new(2, 1));
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 2));

    selection.draw(&mut buffer);

    let modifiers = buffer
        .content()
        .iter()
        .map(|cell| cell.modifier)
        .collect::<Vec<_>>();
    assert_eq!(
        modifiers,
        vec![
            Modifier::empty(),
            Modifier::REVERSED,
            Modifier::REVERSED,
            Modifier::REVERSED,
            Modifier::REVERSED,
            Modifier::REVERSED,
            Modifier::REVERSED,
            Modifier::empty(),
        ]
    );
}
