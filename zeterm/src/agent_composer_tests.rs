use super::{AgentComposer, ComposerMode, ComposerSubmission};
use std::sync::atomic::{AtomicU64, Ordering};
use zeta_editor::{CodeEditorCommand, CodeEditorSelectionMode};
use zeta_input_classifier::InputConversation;
use zeta_ui::{TextInputCompositionCursor, TextInputCompositionEvent};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

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
fn just_task_automatically_switches_to_shell_submission() {
    let root = std::env::temp_dir().join(format!(
        "zeta-agent-composer-{}-{}",
        std::process::id(),
        NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Justfile"), "zeterm-dev:\n    cargo run\n").unwrap();
    let mut composer = AgentComposer::for_working_directory(&root);

    composer.apply(CodeEditorCommand::Insert("just zeterm-dev".to_owned()));

    assert_eq!(composer.mode(), ComposerMode::Shell);
    assert!(matches!(
        composer.submission(),
        Some(ComposerSubmission::ShellCommand(command)) if command == "just zeterm-dev"
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn automatic_mode_uses_the_model_for_command_prefix_questions() {
    let mut composer = AgentComposer::default();

    composer.apply(CodeEditorCommand::Insert(
        "git status 是做什么的".to_owned(),
    ));

    assert_eq!(composer.mode(), ComposerMode::Agent);
    assert!(matches!(
        composer.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "git status 是做什么的"
    ));
}

#[test]
fn automatic_mode_routes_direct_commands_to_shell() {
    let mut composer = AgentComposer::default();

    composer.apply(CodeEditorCommand::Insert("git status".to_owned()));

    assert_eq!(composer.mode(), ComposerMode::Shell);
}

#[test]
fn automatic_mode_keeps_natural_language_one_offs_in_agent() {
    let mut composer = AgentComposer::default();

    composer.apply(CodeEditorCommand::Insert("hello".to_owned()));

    assert_eq!(composer.mode(), ComposerMode::Agent);
}

#[test]
fn explicit_agent_mode_overrides_automatic_shell_detection() {
    let mut composer = AgentComposer::default();
    composer.apply(CodeEditorCommand::Insert("echo explain this".to_owned()));
    assert_eq!(composer.mode(), ComposerMode::Shell);

    composer.set_mode(ComposerMode::Agent);

    assert!(matches!(
        composer.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "echo explain this"
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

#[test]
fn submitting_an_automatic_shell_command_restores_agent_mode() {
    let mut composer = AgentComposer::default();
    composer.apply(CodeEditorCommand::Insert("echo done".to_owned()));
    assert_eq!(composer.mode(), ComposerMode::Shell);

    composer.clear_after_submit();

    assert_eq!(composer.mode(), ComposerMode::Agent);
    assert_eq!(composer.editor().text(), "");
}

#[test]
fn completed_agent_turn_marks_the_next_input_as_a_follow_up() {
    let mut composer = AgentComposer::default();

    composer.mark_agent_message_submitted("fix this");
    composer.mark_agent_turn_completed();
    composer.apply(CodeEditorCommand::Insert("continue".to_owned()));

    assert_eq!(composer.conversation, InputConversation::AgentFollowUp);
    assert_eq!(composer.mode(), ComposerMode::Agent);
}

#[test]
fn shell_turn_completion_does_not_create_agent_follow_up_context() {
    let mut composer = AgentComposer::default();

    composer.mark_shell_command_submitted("cargo test");
    composer.mark_agent_turn_completed();

    assert_eq!(composer.conversation, InputConversation::Standalone);
}

#[test]
fn recent_submission_history_overrides_model_and_shell_allowlists() {
    let mut composer = AgentComposer::default();
    composer.mark_shell_command_submitted("explain this failure");
    composer.set_text("explain this failure");
    assert_eq!(composer.mode(), ComposerMode::Shell);

    composer.clear_after_submit();
    composer.mark_agent_message_submitted("echo productions");
    composer.set_text("echo productions");
    assert_eq!(composer.mode(), ComposerMode::Agent);
}
