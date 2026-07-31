use super::{AgentComposer, ComposerMode, ComposerSubmission};
use zeta_editor::{CodeEditorCommand, CodeEditorSelectionMode};
use zeta_ui::{TextInputCompositionCursor, TextInputCompositionEvent};

#[test]
fn composer_defaults_to_agent_submission() {
    let mut composer = AgentComposer::default();
    composer.apply(CodeEditorCommand::Insert("fix the tests".to_owned()));

    assert!(matches!(
        composer.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "fix the tests"
    ));
}

#[test]
fn shell_mode_produces_an_explicit_shell_submission() {
    let mut composer = AgentComposer::default();
    composer.set_mode(ComposerMode::Shell);
    composer.apply(CodeEditorCommand::Insert("cargo test".to_owned()));

    assert!(matches!(
        composer.submission(),
        Some(ComposerSubmission::ShellCommand(command)) if command == "cargo test"
    ));
}

#[test]
fn switching_mode_cancels_uncommitted_composition_without_losing_text() {
    let mut composer = AgentComposer::default();
    composer.apply(CodeEditorCommand::Insert("committed".to_owned()));
    composer.apply_composition(TextInputCompositionEvent::Preedit {
        text: "候选".to_owned(),
        cursor: TextInputCompositionCursor::Visible(0..6),
    });

    composer.set_mode(ComposerMode::Shell);

    assert_eq!(composer.editor().text(), "committed");
}

#[test]
fn composer_preserves_explicit_newlines_for_multiline_prompts() {
    let mut composer = AgentComposer::default();
    composer.apply(CodeEditorCommand::Insert("explain this".to_owned()));
    composer.apply(CodeEditorCommand::Newline);
    composer.apply(CodeEditorCommand::Insert("src/main.rs".to_owned()));

    assert!(matches!(
        composer.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "explain this\nsrc/main.rs"
    ));
}

#[test]
fn shell_history_replaces_boundary_navigation_and_restores_the_draft() {
    let mut composer = AgentComposer::default();
    composer.set_mode(ComposerMode::Shell);
    composer.apply(CodeEditorCommand::Insert("cargo test".to_owned()));
    composer.clear_after_submit();
    composer.apply(CodeEditorCommand::Insert("draft".to_owned()));

    composer.apply(CodeEditorCommand::MoveUp(CodeEditorSelectionMode::Move));
    assert_eq!(composer.editor().text(), "cargo test");

    composer.apply(CodeEditorCommand::MoveDown(CodeEditorSelectionMode::Move));
    assert_eq!(composer.editor().text(), "draft");
}
