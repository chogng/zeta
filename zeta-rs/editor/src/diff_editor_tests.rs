use super::{DiffEditor, DiffEditorLabels, DiffEditorSide, DiffEditorState, DiffEditorStyle};
use zeta_diff::DiffDocument;
use zeta_ui::{Color, Component, Point, Rect, UiScene};

fn document() -> DiffDocument {
    DiffDocument::from_text(
        "same\nlet value = 1;\nremoved\n",
        "same\nlet value = 20;\nadded\nextra\n",
    )
    .unwrap()
}

fn editor<'a>(document: &'a DiffDocument, bounds: Rect, state: DiffEditorState) -> DiffEditor<'a> {
    DiffEditor::new(
        bounds,
        document,
        state,
        DiffEditorLabels::new("Original · src/main.rs", "Modified · src/main.rs"),
        DiffEditorStyle::light(),
    )
}

#[test]
fn state_clamps_synchronized_vertical_scroll_and_keeps_horizontal_sides_independent() {
    let mut state = DiffEditorState::default();

    state.scroll_rows(6, 10, 3);
    state.set_horizontal_column(DiffEditorSide::Original, 4);
    state.set_horizontal_column(DiffEditorSide::Modified, 11);

    assert_eq!(state.first_visible_row(), 6);
    assert_eq!(state.horizontal_column(DiffEditorSide::Original), 4);
    assert_eq!(state.horizontal_column(DiffEditorSide::Modified), 11);

    state.scroll_rows(20, 10, 3);
    assert_eq!(state.first_visible_row(), 7);
    state.scroll_rows(-2, 10, 3);
    assert_eq!(state.first_visible_row(), 5);
    state.clamp(4, 3);
    assert_eq!(state.first_visible_row(), 1);
}

#[test]
fn side_by_side_paint_includes_headers_mapped_lines_markers_and_inline_highlights() {
    let document = document();
    let editor = editor(
        &document,
        Rect::from_xywh(10.0, 20.0, 640.0, 240.0),
        DiffEditorState::default(),
    );
    let style = DiffEditorStyle::light();
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    let texts = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(texts.contains(&"Original · src/main.rs"));
    assert!(texts.contains(&"Modified · src/main.rs"));
    assert!(texts.contains(&"same"));
    assert!(texts.contains(&"let value = 1;"));
    assert!(texts.contains(&"let value = 20;"));
    assert!(texts.contains(&"−"));
    assert!(texts.contains(&"+"));
    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == style.removed_line)
    );
    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == style.added_line)
    );
    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == style.removed_inline)
    );
    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == style.added_inline)
    );
}

#[test]
fn location_maps_both_panes_to_the_same_visible_diff_row() {
    let document = document();
    let editor = editor(
        &document,
        Rect::from_xywh(0.0, 0.0, 601.0, 92.0),
        DiffEditorState::default(),
    );
    let layout = editor.layout();
    let y = layout.original.origin.y + 32.0 + 20.0 + 2.0;

    let original = editor
        .location_at(Point::new(layout.original.origin.x + 2.0, y))
        .unwrap();
    let modified = editor
        .location_at(Point::new(layout.modified.origin.x + 2.0, y))
        .unwrap();

    assert_eq!(original.side, DiffEditorSide::Original);
    assert_eq!(modified.side, DiffEditorSide::Modified);
    assert_eq!(original.row_index, modified.row_index);
    assert_eq!(original.line_number, Some(2));
    assert_eq!(modified.line_number, Some(2));
    assert_eq!(editor.location_at(Point::new(10.0, 10.0)), None);
}

#[test]
fn visible_range_never_paints_more_rows_than_the_body_capacity() {
    let document = document();
    let mut state = DiffEditorState::default();
    state.scroll_rows(isize::MAX, document.rows().len(), 2);
    let editor = editor(&document, Rect::from_xywh(0.0, 0.0, 400.0, 72.0), state);

    assert_eq!(editor.visible_row_capacity(), 2);
    assert_eq!(editor.visible_row_range().len(), 2);
    assert_eq!(editor.visible_row_range().end, document.rows().len());
}

#[test]
fn absent_counterpart_uses_missing_line_background_without_fake_line_number() {
    let document = DiffDocument::from_text("", "new line\n").unwrap();
    let editor = editor(
        &document,
        Rect::from_xywh(0.0, 0.0, 400.0, 80.0),
        DiffEditorState::default(),
    );
    let style = DiffEditorStyle::light();
    let layout = editor.layout();
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == style.missing_line)
    );
    assert_eq!(
        editor
            .location_at(Point::new(
                layout.original.origin.x + 2.0,
                layout.original.origin.y + 32.0 + 2.0
            ))
            .unwrap()
            .line_number,
        None
    );
}
