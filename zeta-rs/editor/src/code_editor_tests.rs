use super::text_metrics::{display_columns_until, visit_display_cell_runs};
use super::{
    CodeEditor, CodeEditorCommand, CodeEditorDocument, CodeEditorHeader, CodeEditorInlineHighlight,
    CodeEditorPresentation, CodeEditorRow, CodeEditorRowSource, CodeEditorSelectionMode,
    CodeEditorStyle, CodeEditorSyntaxHighlighter, CodeEditorSyntaxToken, CodeEditorViewport,
};
use zeta_ui::{CaretVisibility, Color, Component, Point, Rect, TextBlockWrap, UiScene};
use zeta_ui::{TextInputCompositionCursor, TextInputCompositionEvent};

#[test]
fn document_indexes_lf_crlf_cr_and_replacement_without_rendering_terminators() {
    let mut document = CodeEditorDocument::from_text("one\r\ntwo\nthree\rfour");

    assert_eq!(document.row_count(), 4);
    assert_eq!(document.row(0).unwrap().text, Some("one"));
    assert_eq!(document.row(1).unwrap().text, Some("two"));
    assert_eq!(document.row(2).unwrap().text, Some("three"));
    assert_eq!(document.row(3).unwrap().text, Some("four"));
    assert_eq!(document.text(), "one\r\ntwo\nthree\rfour");

    document.replace_text("replacement\n");
    assert_eq!(document.row_count(), 2);
    assert_eq!(document.row(0).unwrap().text, Some("replacement"));
    assert_eq!(document.row(1).unwrap().text, Some(""));
}

#[test]
fn code_editor_paints_only_visible_numbered_rows_and_optional_header() {
    let document = CodeEditorDocument::from_text("one\ntwo\nthree\nfour\n");
    let editor = CodeEditor::new(
        Rect::from_xywh(10.0, 20.0, 320.0, 72.0),
        &document,
        CodeEditorViewport::new(1),
        CodeEditorHeader::Label("src/main.rs"),
        CodeEditorStyle::light(),
    );
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    assert_eq!(editor.visible_row_capacity(), 2);
    assert_eq!(editor.visible_row_range(), 1..3);
    let texts = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(texts.contains(&"src/main.rs"));
    assert!(texts.contains(&"two"));
    assert!(texts.contains(&"three"));
    assert!(!texts.contains(&"one"));
    assert!(!texts.contains(&"four"));
}

#[test]
fn empty_rows_do_not_emit_zero_width_text_blocks() {
    let document = CodeEditorDocument::from_text("one\n\nthree");
    let editor = CodeEditor::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 60.0),
        &document,
        CodeEditorViewport::default(),
        CodeEditorHeader::Hidden,
        CodeEditorStyle::light(),
    );
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    assert!(scene.text_blocks().iter().all(|block| {
        !block.text().is_empty() && block.bounds().width > 0.0 && block.bounds().height > 0.0
    }));
    assert_eq!(
        scene
            .text_blocks()
            .iter()
            .filter(|block| matches!(block.text(), "one" | "three"))
            .count(),
        2
    );
}

#[test]
fn compact_presentation_uses_the_full_width_without_line_number_chrome() {
    let document = CodeEditorDocument::from_text("hello");
    let editor = CodeEditor::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 40.0),
        &document,
        CodeEditorViewport::default(),
        CodeEditorHeader::Hidden,
        CodeEditorStyle::light(),
    )
    .with_presentation(CodeEditorPresentation::Compact)
    .with_caret_visibility(CaretVisibility::Hidden);
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    let hello = scene
        .text_blocks()
        .iter()
        .find(|block| block.text() == "hello")
        .unwrap();
    assert_eq!(hello.origin().x, 8.0);
    assert!(scene.text_blocks().iter().all(|block| block.text() != "1"));
    assert!(editor.caret_bounds().is_some());
}

#[test]
fn code_rows_keep_chinese_text_and_spaces_on_one_unwrapped_source_line() {
    let text = "中文 空格";
    let mut document = CodeEditorDocument::from_text(text);
    document.apply(CodeEditorCommand::SelectAll);
    document.apply(CodeEditorCommand::MoveRight(CodeEditorSelectionMode::Move));
    let editor = CodeEditor::new(
        Rect::from_xywh(0.0, 0.0, 220.0, 40.0),
        &document,
        CodeEditorViewport::default(),
        CodeEditorHeader::Hidden,
        CodeEditorStyle::light(),
    )
    .with_presentation(CodeEditorPresentation::Compact);
    let caret = editor.caret_bounds().unwrap();
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    let glyphs = scene
        .text_blocks()
        .iter()
        .map(|block| {
            (
                block.text(),
                block.origin().x,
                block.bounds().width,
                block.wrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        glyphs,
        vec![
            ("中", 8.0, 16.0, TextBlockWrap::None),
            ("文", 24.0, 16.0, TextBlockWrap::None),
            ("空", 48.0, 16.0, TextBlockWrap::None),
            ("格", 64.0, 16.0, TextBlockWrap::None),
        ]
    );
    assert_eq!(caret.origin.x, 80.0);
    assert_eq!(document.text(), text);
}

#[test]
fn every_unicode_scalar_preserves_spaces_in_the_display_cell_projection() {
    for codepoint in 0..=char::MAX as u32 {
        let Some(character) = char::from_u32(codepoint) else {
            continue;
        };
        let text = super::expand_tabs(&format!("界 {character} 界"));
        let expected_columns = super::display_columns(&text);
        let mut previous_right = 0;
        let projected_columns = visit_display_cell_runs(&text, |run| {
            assert!(
                run.column >= previous_right,
                "overlapping display-cell runs for U+{codepoint:04X}"
            );
            assert_eq!(
                run.columns,
                super::display_columns(run.text),
                "run width mismatch for U+{codepoint:04X}"
            );
            assert!(
                run.column + run.columns <= expected_columns,
                "run exceeds the caret column for U+{codepoint:04X}"
            );
            previous_right = run.column + run.columns;
        });
        assert_eq!(
            projected_columns, expected_columns,
            "text and caret columns diverged for U+{codepoint:04X}"
        );
    }
}

#[test]
fn multi_scalar_graphemes_keep_text_spaces_and_caret_in_one_projection() {
    let samples = [
        "e\u{301}",
        "क्",
        "क्ष",
        "กำ",
        "한",
        "👩‍💻",
        "👨‍👩‍👧‍👦",
        "👍🏽",
        "🇨🇳",
        "1️⃣",
        "♥︎",
        "♥️",
        "\u{200d}",
        "\u{2067}עברית\u{2069}",
        "\u{3000}",
    ];

    for sample in samples {
        let text = format!("界 {sample} 界");
        let mut document = CodeEditorDocument::from_text(&text);
        document.apply(CodeEditorCommand::SelectAll);
        document.apply(CodeEditorCommand::MoveRight(CodeEditorSelectionMode::Move));
        let editor = CodeEditor::new(
            Rect::from_xywh(0.0, 0.0, 480.0, 40.0),
            &document,
            CodeEditorViewport::default(),
            CodeEditorHeader::Hidden,
            CodeEditorStyle::light(),
        )
        .with_presentation(CodeEditorPresentation::Compact);
        let mut scene = UiScene::new(Color::WHITE);

        editor.paint(&mut scene);

        assert_eq!(document.text(), text, "document changed for {sample:?}");
        assert_eq!(
            editor.caret_bounds().unwrap().origin.x,
            8.0 + super::display_columns(&text) as f32 * 8.0,
            "caret drifted for {sample:?}"
        );
        assert!(
            scene
                .text_blocks()
                .iter()
                .all(|block| block.wrap() == TextBlockWrap::None),
            "a text run wrapped for {sample:?}"
        );
    }
}

#[test]
fn viewport_clamps_vertical_scroll_and_retains_horizontal_column() {
    let mut viewport = CodeEditorViewport::default();

    viewport.scroll_rows(20, 10, 3);
    viewport.set_horizontal_column(12);

    assert_eq!(viewport.first_visible_row(), 7);
    assert_eq!(viewport.horizontal_column(), 12);
    viewport.scroll_rows(-2, 10, 3);
    assert_eq!(viewport.first_visible_row(), 5);
    viewport.clamp(4, 3);
    assert_eq!(viewport.first_visible_row(), 1);
}

#[test]
fn viewport_reveals_rows_above_and_below_the_retained_window() {
    let mut viewport = CodeEditorViewport::new(3);

    viewport.reveal_row(8, 12, 4);
    assert_eq!(viewport.first_visible_row(), 5);
    viewport.reveal_row(2, 12, 4);
    assert_eq!(viewport.first_visible_row(), 2);
}

#[test]
fn location_maps_visible_geometry_back_to_document_rows() {
    let document = CodeEditorDocument::from_text("one\ntwo\nthree\n");
    let editor = CodeEditor::new(
        Rect::from_xywh(0.0, 0.0, 300.0, 40.0),
        &document,
        CodeEditorViewport::new(1),
        CodeEditorHeader::Hidden,
        CodeEditorStyle::light(),
    );

    let location = editor.location_at(Point::new(10.0, 4.0)).unwrap();

    assert_eq!(location.row_index, 1);
    assert_eq!(location.line_number, Some(2));
    assert_eq!(editor.location_at(Point::new(10.0, 45.0)), None);
    assert_eq!(editor.caret_bounds(), None);
}

#[test]
fn unicode_width_and_tabs_share_inline_column_mapping() {
    let text = "a\t界👩‍💻z";
    let emoji_end = text.find('z').unwrap();

    assert_eq!(super::expand_tabs(text), "a   界👩‍💻z");
    assert_eq!(display_columns_until(text, "a\t".len()), 4);
    assert_eq!(display_columns_until(text, emoji_end), 8);
    assert_eq!(super::display_columns(text), 9);
}

struct DecoratedRows;

impl CodeEditorRowSource for DecoratedRows {
    fn row_count(&self) -> usize {
        1
    }

    fn largest_line_number(&self) -> usize {
        1
    }

    fn row(&self, index: usize) -> Option<CodeEditorRow<'_>> {
        (index == 0).then(|| {
            CodeEditorRow::new(1, "let value = 1;").with_inline_highlights(vec![
                CodeEditorInlineHighlight::new(4..9, Color::rgb(220, 230, 255)),
                CodeEditorInlineHighlight::new(1..usize::MAX, Color::rgb(255, 0, 0)),
            ])
        })
    }
}

#[test]
fn row_source_decorations_are_shared_and_invalid_utf8_ranges_are_ignored() {
    let editor = CodeEditor::new(
        Rect::from_xywh(0.0, 0.0, 300.0, 40.0),
        &DecoratedRows,
        CodeEditorViewport::default(),
        CodeEditorHeader::Hidden,
        CodeEditorStyle::light(),
    );
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == Color::rgb(220, 230, 255))
    );
    assert!(
        !scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == Color::rgb(255, 0, 0))
    );
}

#[test]
fn multiline_keyboard_editing_preserves_graphemes_and_vertical_columns() {
    let mut document = CodeEditorDocument::from_text("hello\nworld");
    document.apply(CodeEditorCommand::MoveToLineEnd(
        CodeEditorSelectionMode::Move,
    ));
    document.apply(CodeEditorCommand::MoveDown(CodeEditorSelectionMode::Move));
    assert_eq!(document.cursor(), document.text().len());

    document.apply(CodeEditorCommand::Insert(" 👩‍💻".into()));
    document.apply(CodeEditorCommand::Backspace);
    assert_eq!(document.text(), "hello\nworld ");

    document.apply(CodeEditorCommand::MoveUp(CodeEditorSelectionMode::Move));
    assert_eq!(document.cursor(), 5);
    document.apply(CodeEditorCommand::Newline);
    assert_eq!(document.text(), "hello\n\nworld ");
}

#[test]
fn selection_replacement_and_undo_redo_are_atomic() {
    let mut document = CodeEditorDocument::from_text("hello");
    document.apply(CodeEditorCommand::MoveRight(
        CodeEditorSelectionMode::Extend,
    ));
    document.apply(CodeEditorCommand::MoveRight(
        CodeEditorSelectionMode::Extend,
    ));
    assert_eq!(document.selected_text(), Some("he"));

    document.apply(CodeEditorCommand::Insert("HE".into()));
    assert_eq!(document.text(), "HEllo");
    assert!(document.can_undo());
    document.apply(CodeEditorCommand::Undo);
    assert_eq!(document.text(), "hello");
    assert_eq!(document.selected_text(), Some("he"));
    assert!(document.can_redo());
    document.apply(CodeEditorCommand::Redo);
    assert_eq!(document.text(), "HEllo");
}

#[test]
fn ime_preedit_is_uncommitted_until_commit_and_commit_is_undoable() {
    let mut document = CodeEditorDocument::from_text("let value = ");
    document.apply(CodeEditorCommand::MoveToLineEnd(
        CodeEditorSelectionMode::Move,
    ));
    document.apply_composition(TextInputCompositionEvent::Preedit {
        text: "ni".into(),
        cursor: TextInputCompositionCursor::Visible(2..2),
    });

    assert_eq!(document.text(), "let value = ");
    assert_eq!(document.composition().unwrap().text, "ni");

    document.apply_composition(TextInputCompositionEvent::Commit("你".into()));
    assert_eq!(document.text(), "let value = 你");
    assert_eq!(document.composition(), None);
    document.apply(CodeEditorCommand::Undo);
    assert_eq!(document.text(), "let value = ");
}

struct RustKeywordHighlighter;

impl CodeEditorSyntaxHighlighter for RustKeywordHighlighter {
    fn highlight_line(&self, _line_number: usize, text: &str) -> Vec<CodeEditorSyntaxToken> {
        text.starts_with("let")
            .then(|| CodeEditorSyntaxToken::new(0..3, Color::rgb(130, 80, 223)))
            .into_iter()
            .collect()
    }
}

#[test]
fn syntax_tokens_selection_caret_and_preedit_are_projected_into_the_scene() {
    let mut document = CodeEditorDocument::from_text("let value = 1;");
    document.apply_syntax(&RustKeywordHighlighter);
    document.apply(CodeEditorCommand::MoveRight(
        CodeEditorSelectionMode::Extend,
    ));
    document.apply_composition(TextInputCompositionEvent::Preedit {
        text: "x".into(),
        cursor: TextInputCompositionCursor::Visible(1..1),
    });
    let editor = CodeEditor::new(
        Rect::from_xywh(0.0, 0.0, 320.0, 40.0),
        &document,
        CodeEditorViewport::default(),
        CodeEditorHeader::Hidden,
        CodeEditorStyle::light(),
    );
    let mut scene = UiScene::new(Color::WHITE);

    editor.paint(&mut scene);

    assert!(editor.caret_bounds().is_some());
    assert!(scene.text_blocks().iter().any(|block| {
        block.text() == "let" && block.style().color() == Color::rgb(130, 80, 223)
    }));
    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == Color::rgba(68, 139, 202, 72))
    );
    assert!(
        scene
            .rects()
            .iter()
            .any(|rect| rect.fill() == Color::rgb(15, 110, 96))
    );
    assert!(scene.text_blocks().iter().any(|block| block.text() == "x"));
}
