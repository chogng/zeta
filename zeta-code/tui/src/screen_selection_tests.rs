use super::ClickCount;
use super::ScreenSelection;
use super::ScreenSelectionOutcome;
use super::ScreenSelectionRange;
use super::line_range_at;
use super::text_in_range;
use super::token_range_at;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use std::time::Duration;
use std::time::Instant;

#[test]
fn drag_selection_normalizes_both_directions_and_remains_visible_after_release() {
    let mut selection = ScreenSelection::default();
    let now = Instant::now();
    selection.begin(Position::new(8, 3));
    selection.drag(Position::new(2, 1));

    assert_eq!(
        selection.finish(Position::new(2, 1), now),
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
        selection.finish(position, Instant::now()),
        Some(ScreenSelectionOutcome::Click {
            position,
            count: ClickCount::Single,
        })
    );
    assert_eq!(selection.range(), None);
}

#[test]
fn nearby_clicks_cycle_through_single_double_and_triple() {
    let mut selection = ScreenSelection::default();
    let started = Instant::now();

    assert_click(
        &mut selection,
        Position::new(4, 2),
        started,
        ClickCount::Single,
    );
    assert_click(
        &mut selection,
        Position::new(5, 2),
        started + Duration::from_millis(100),
        ClickCount::Double,
    );
    selection.select(ScreenSelectionRange::new(
        Position::new(2, 2),
        Position::new(7, 2),
    ));
    assert_click(
        &mut selection,
        Position::new(4, 2),
        started + Duration::from_millis(200),
        ClickCount::Triple,
    );
    assert_click(
        &mut selection,
        Position::new(4, 2),
        started + Duration::from_millis(300),
        ClickCount::Single,
    );
}

#[test]
fn slow_click_or_drag_starts_a_new_click_sequence() {
    let mut selection = ScreenSelection::default();
    let started = Instant::now();
    assert_click(
        &mut selection,
        Position::new(4, 2),
        started,
        ClickCount::Single,
    );
    assert_click(
        &mut selection,
        Position::new(4, 2),
        started + Duration::from_millis(501),
        ClickCount::Single,
    );

    selection.begin(Position::new(4, 2));
    selection.drag(Position::new(8, 2));
    assert!(matches!(
        selection.finish(Position::new(8, 2), started + Duration::from_millis(600)),
        Some(ScreenSelectionOutcome::Copy(_))
    ));
    assert_click(
        &mut selection,
        Position::new(4, 2),
        started + Duration::from_millis(700),
        ClickCount::Single,
    );
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
fn token_selection_distinguishes_words_spaces_symbols_and_wide_characters() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 24, 1));
    buffer.set_string(0, 0, "alpha  beta::你好吗", Style::default());

    assert_eq!(selected_token(&buffer, 2), Some("alpha".into()));
    assert_eq!(selected_token(&buffer, 5), Some("  ".into()));
    assert_eq!(selected_token(&buffer, 11), Some("::".into()));
    assert_eq!(selected_token(&buffer, 14), Some("你好吗".into()));
}

#[test]
fn line_selection_uses_the_visual_row_and_trims_terminal_padding() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 2));
    buffer.set_string(0, 1, "  alpha", Style::default());

    let range = line_range_at(&buffer, Position::new(4, 1)).unwrap();

    assert_eq!(text_in_range(&buffer, range), Some("  alpha".into()));
}

#[test]
fn selection_highlight_applies_to_every_rendered_surface_cell_in_the_range() {
    let mut selection = ScreenSelection::default();
    selection.begin(Position::new(1, 0));
    selection.drag(Position::new(2, 1));
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 2));

    selection.draw(&mut buffer, crate::render::test_context());

    let colors = buffer
        .content()
        .iter()
        .map(|cell| (cell.fg, cell.bg))
        .collect::<Vec<_>>();
    assert_eq!(
        colors,
        vec![
            (Color::Reset, Color::Reset),
            (Color::Rgb(13, 17, 23), Color::Rgb(135, 206, 235)),
            (Color::Rgb(13, 17, 23), Color::Rgb(135, 206, 235)),
            (Color::Rgb(13, 17, 23), Color::Rgb(135, 206, 235)),
            (Color::Rgb(13, 17, 23), Color::Rgb(135, 206, 235)),
            (Color::Rgb(13, 17, 23), Color::Rgb(135, 206, 235)),
            (Color::Rgb(13, 17, 23), Color::Rgb(135, 206, 235)),
            (Color::Reset, Color::Reset),
        ]
    );
}

fn assert_click(
    selection: &mut ScreenSelection,
    position: Position,
    now: Instant,
    expected: ClickCount,
) {
    selection.begin(position);
    assert_eq!(
        selection.finish(position, now),
        Some(ScreenSelectionOutcome::Click {
            position,
            count: expected,
        })
    );
}

fn selected_token(buffer: &Buffer, column: u16) -> Option<String> {
    token_range_at(buffer, Position::new(column, 0)).and_then(|range| text_in_range(buffer, range))
}
