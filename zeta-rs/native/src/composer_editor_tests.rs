use super::{ComposerEditor, ComposerEditorFocus};
use zeta_editor::CodeEditorCommand;
use zeta_ui::{CaretVisibility, Color, Component, Rect, UiScene};

#[test]
fn compact_editor_grows_until_eight_visible_rows() {
    let mut editor = ComposerEditor::default();
    assert_eq!(editor.preferred_height(), 44.0);

    editor.apply(CodeEditorCommand::Insert("one\ntwo\nthree".to_owned()));
    assert_eq!(editor.visible_row_count(), 3);
    assert_eq!(editor.preferred_height(), 60.0);

    editor.set_text(
        (0..12)
            .map(|row| format!("row {row}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    assert_eq!(editor.visible_row_count(), 8);
    assert_eq!(editor.preferred_height(), 160.0);
}

#[test]
fn empty_editor_paints_placeholder_and_only_exposes_a_focused_visible_caret() {
    let editor = ComposerEditor::default();
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, editor.preferred_height());
    let mut scene = UiScene::new(Color::WHITE);
    let blurred = editor.view(
        bounds,
        "Ask Zeta anything…",
        ComposerEditorFocus::Blurred,
        Color::rgb(126, 126, 132),
    );

    assert_eq!(blurred.caret_bounds(), None);
    blurred.paint(&mut scene);
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "Ask Zeta anything…")
    );

    let focused = editor.view(
        bounds,
        "Ask Zeta anything…",
        ComposerEditorFocus::Focused(CaretVisibility::Visible),
        Color::rgb(126, 126, 132),
    );
    assert!(focused.caret_bounds().is_some());
}
