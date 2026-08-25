use super::{ComposerInput, ComposerInputFocus};
use zeta_editor::{CodeEditorCommand, CodeEditorLanguage};
use zeta_ui::{CaretVisibility, Color, Component, Rect, UiScene};

#[test]
fn compact_editor_grows_until_eight_visible_rows() {
    let mut editor = ComposerInput::default();
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
fn page_navigation_uses_the_composer_visible_row_cap() {
    let mut editor = ComposerInput::default();
    editor.set_text(
        (0..10)
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    );

    editor.apply(CodeEditorCommand::MovePageUp(
        zeta_editor::CodeEditorSelectionMode::Move,
    ));
    editor.apply(CodeEditorCommand::Insert("x".to_owned()));

    assert!(editor.text().starts_with("0\n1x\n2"));
}

#[test]
fn empty_editor_paints_placeholder_and_only_exposes_a_focused_visible_caret() {
    let editor = ComposerInput::default();
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, editor.preferred_height());
    let mut scene = UiScene::new(Color::WHITE);
    let blurred = editor.view(
        bounds,
        "Ask Zeta anything…",
        ComposerInputFocus::Blurred,
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
        ComposerInputFocus::Focused(CaretVisibility::Visible),
        Color::rgb(126, 126, 132),
    );
    assert!(focused.caret_bounds().is_some());
}

#[test]
fn shell_highlighting_projects_syntax_tokens_into_code_editor() {
    let mut editor = ComposerInput::default();
    editor.set_text("just zeterm-dev");
    editor.set_language(CodeEditorLanguage::Shell);
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, editor.preferred_height());
    let mut scene = UiScene::new(Color::WHITE);

    editor
        .view(
            bounds,
            "",
            ComposerInputFocus::Blurred,
            Color::rgb(126, 126, 132),
        )
        .paint(&mut scene);

    let command = scene
        .text_blocks()
        .iter()
        .find(|block| block.text() == "just")
        .expect("syntax command token should be painted");
    assert_eq!(command.style().color(), Color::rgb(15, 110, 96));
    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "just zeterm-dev")
    );
}

#[test]
fn focused_composer_projects_ghost_text_without_committing_it() {
    let mut editor = ComposerInput::default();
    editor.set_text("git ch");
    editor.show_ghost_text("eckout".to_owned());
    let bounds = Rect::from_xywh(0.0, 0.0, 320.0, editor.preferred_height());
    let mut scene = UiScene::new(Color::WHITE);

    editor
        .view(
            bounds,
            "",
            ComposerInputFocus::Focused(CaretVisibility::Visible),
            Color::rgb(126, 126, 132),
        )
        .paint(&mut scene);

    assert!(
        scene
            .text_blocks()
            .iter()
            .any(|block| block.text() == "eckout")
    );
    assert_eq!(editor.text(), "git ch");

    let mut blurred_scene = UiScene::new(Color::WHITE);
    editor
        .view(
            bounds,
            "",
            ComposerInputFocus::Blurred,
            Color::rgb(126, 126, 132),
        )
        .paint(&mut blurred_scene);
    assert!(
        blurred_scene
            .text_blocks()
            .iter()
            .all(|block| block.text() != "eckout")
    );
}
