use super::ChatInput;
use super::ComposerRoute;
use super::ComposerSubmission;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use zeta_editor::CodeEditorCommand;
use zeta_editor::CodeEditorLanguage;
use zeta_editor::CodeEditorSelectionMode;
use zeta_input_classifier::InputConversation;
use zeta_ui_components::ScrollCommand;
use zeta_ui_components::ScrollDelta;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::Size;
use zui::ui::TextInputCompositionCursor;
use zui::ui::TextInputCompositionEvent;

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn chat_input_defaults_to_agent_submission() {
    let mut chat_input = ChatInput::default();
    chat_input.apply(CodeEditorCommand::Insert("fix the tests".to_owned()));

    assert!(matches!(
        chat_input.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "fix the tests"
    ));
}

#[test]
fn classifier_routes_a_direct_command_to_shell_submission() {
    let mut chat_input = ChatInput::default();
    chat_input.apply(CodeEditorCommand::Insert("cargo test".to_owned()));

    assert!(matches!(
        chat_input.submission(),
        Some(ComposerSubmission::ShellCommand(command)) if command == "cargo test"
    ));
}

#[test]
fn classifier_routes_a_just_task_to_shell_submission() {
    let root = std::env::temp_dir().join(format!(
        "zeta-agent-chat_input-{}-{}",
        std::process::id(),
        NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Justfile"), "app:\n    cargo run\n").unwrap();
    let mut chat_input = ChatInput::for_working_directory(&root);

    chat_input.apply(CodeEditorCommand::Insert("just app".to_owned()));

    assert_eq!(chat_input.route(), ComposerRoute::Shell);
    assert!(matches!(
        chat_input.submission(),
        Some(ComposerSubmission::ShellCommand(command)) if command == "just app"
    ));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn classifier_uses_the_model_for_command_prefix_questions() {
    let mut chat_input = ChatInput::default();

    chat_input.apply(CodeEditorCommand::Insert(
        "git status 是做什么的".to_owned(),
    ));

    assert_eq!(chat_input.route(), ComposerRoute::Agent);
    assert_eq!(chat_input.input().language(), CodeEditorLanguage::PlainText);
    assert!(matches!(
        chat_input.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "git status 是做什么的"
    ));
}

#[test]
fn only_a_whole_shell_submission_uses_shell_highlighting() {
    let mut chat_input = ChatInput::default();
    chat_input.set_text("git status");
    assert_eq!(chat_input.route(), ComposerRoute::Shell);
    assert_eq!(chat_input.input().language(), CodeEditorLanguage::Shell);

    chat_input.set_text("git status 是做什么的");

    assert_eq!(chat_input.route(), ComposerRoute::Agent);
    assert_eq!(chat_input.input().language(), CodeEditorLanguage::PlainText);
}

#[test]
fn classifier_routes_direct_commands_to_shell() {
    let mut chat_input = ChatInput::default();

    chat_input.apply(CodeEditorCommand::Insert("git status".to_owned()));

    assert_eq!(chat_input.route(), ComposerRoute::Shell);
}

#[test]
fn classified_shell_route_offers_command_prefix_completions() {
    let mut chat_input = ChatInput::default();
    chat_input.set_text("ech");

    assert_eq!(chat_input.input().ghost_text(), Some("o"));
    assert!(chat_input.accept_shell_suggestion());
    assert_eq!(chat_input.input().text(), "echo");
    assert_eq!(chat_input.route(), ComposerRoute::Shell);
    assert_eq!(chat_input.input().ghost_text(), None);
}

#[test]
fn shell_ghost_text_is_not_offered_when_the_caret_is_inside_a_token() {
    let mut chat_input = ChatInput::default();
    chat_input.set_text("git cheout");
    assert_eq!(chat_input.route(), ComposerRoute::Shell);
    for _ in 0..3 {
        chat_input.apply(CodeEditorCommand::MoveLeft(CodeEditorSelectionMode::Move));
    }

    assert!(!chat_input.has_shell_suggestion());
    assert_eq!(chat_input.input().text(), "git cheout");
}

#[test]
fn shell_ghost_text_accepts_only_the_common_prefix_of_multiple_paths() {
    let root = std::env::temp_dir().join(format!(
        "zeta-agent-chat_input-completion-{}-{}",
        std::process::id(),
        NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("alpha-one"), "").unwrap();
    std::fs::write(root.join("alpha-two"), "").unwrap();
    let mut chat_input = ChatInput::for_working_directory(&root);
    chat_input.set_text("cat al");

    assert_eq!(chat_input.input().ghost_text(), Some("pha-"));
    assert!(chat_input.accept_shell_suggestion());
    assert_eq!(chat_input.input().text(), "cat alpha-");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn dismissed_shell_ghost_text_stays_hidden_until_input_changes() {
    let mut chat_input = ChatInput::default();
    chat_input.set_text("ech");

    assert!(chat_input.dismiss_shell_suggestion());
    assert_eq!(chat_input.input().ghost_text(), None);
    chat_input.cancel_composition();
    assert_eq!(chat_input.input().ghost_text(), None);

    chat_input.apply(CodeEditorCommand::Insert("o".to_owned()));
    assert_eq!(chat_input.input().text(), "echo");
}

#[test]
fn classifier_keeps_natural_language_one_offs_in_agent() {
    let mut chat_input = ChatInput::default();

    chat_input.apply(CodeEditorCommand::Insert("hello".to_owned()));

    assert_eq!(chat_input.route(), ComposerRoute::Agent);
}

#[test]
fn classification_is_recomputed_when_the_input_changes() {
    let mut chat_input = ChatInput::default();
    chat_input.set_text("git status");
    assert_eq!(chat_input.route(), ComposerRoute::Shell);

    chat_input.set_text("git status 是做什么的");

    assert_eq!(chat_input.route(), ComposerRoute::Agent);
    assert!(matches!(
        chat_input.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "git status 是做什么的"
    ));
    assert!(!chat_input.has_shell_suggestion());
}

#[test]
fn active_ime_composition_suppresses_shell_ghost_text() {
    let mut chat_input = ChatInput::default();
    chat_input.set_text("ech");
    assert!(chat_input.has_shell_suggestion());

    chat_input.apply_composition(TextInputCompositionEvent::Preedit {
        text: "o".to_owned(),
        cursor: TextInputCompositionCursor::Visible(0..1),
    });

    assert!(!chat_input.has_shell_suggestion());
    assert_eq!(chat_input.input().ghost_text(), None);
}

#[test]
fn chat_input_preserves_explicit_newlines_for_multiline_prompts() {
    let mut chat_input = ChatInput::default();
    chat_input.apply(CodeEditorCommand::Insert("explain this".to_owned()));
    chat_input.apply(CodeEditorCommand::Newline);
    chat_input.apply(CodeEditorCommand::Insert("src/main.rs".to_owned()));

    assert!(matches!(
        chat_input.submission(),
        Some(ComposerSubmission::AgentMessage(text)) if text == "explain this\nsrc/main.rs"
    ));
}

#[test]
fn shell_history_replaces_boundary_navigation_and_restores_the_draft() {
    let mut chat_input = ChatInput::default();
    chat_input.apply(CodeEditorCommand::Insert("cargo test".to_owned()));
    chat_input.clear_after_submit();
    chat_input.apply(CodeEditorCommand::Insert("git diff".to_owned()));

    chat_input.apply(CodeEditorCommand::MoveUp(CodeEditorSelectionMode::Move));
    assert_eq!(chat_input.input().text(), "cargo test");

    chat_input.apply(CodeEditorCommand::MoveDown(CodeEditorSelectionMode::Move));
    assert_eq!(chat_input.input().text(), "git diff");
}

#[test]
fn submitting_a_shell_command_restores_the_default_agent_route() {
    let mut chat_input = ChatInput::default();
    chat_input.apply(CodeEditorCommand::Insert("echo done".to_owned()));
    assert_eq!(chat_input.route(), ComposerRoute::Shell);

    chat_input.clear_after_submit();

    assert_eq!(chat_input.route(), ComposerRoute::Agent);
    assert_eq!(chat_input.input().text(), "");
}

#[test]
fn completed_agent_turn_marks_the_next_input_as_a_follow_up() {
    let mut chat_input = ChatInput::default();

    chat_input.mark_agent_message_submitted("fix this");
    chat_input.mark_agent_turn_completed();
    chat_input.apply(CodeEditorCommand::Insert("continue".to_owned()));

    assert_eq!(chat_input.conversation, InputConversation::AgentFollowUp);
    assert_eq!(chat_input.route(), ComposerRoute::Agent);
}

#[test]
fn shell_turn_completion_does_not_create_agent_follow_up_context() {
    let mut chat_input = ChatInput::default();

    chat_input.mark_shell_command_submitted("cargo test");
    chat_input.mark_agent_turn_completed();

    assert_eq!(chat_input.conversation, InputConversation::Standalone);
}

#[test]
fn recent_submission_history_overrides_model_and_shell_allowlists() {
    let mut chat_input = ChatInput::default();
    chat_input.mark_shell_command_submitted("explain this failure");
    chat_input.set_text("explain this failure");
    assert_eq!(chat_input.route(), ComposerRoute::Shell);

    chat_input.clear_after_submit();
    chat_input.mark_agent_message_submitted("echo productions");
    chat_input.set_text("echo productions");
    assert_eq!(chat_input.route(), ComposerRoute::Agent);
}

#[test]
fn interaction_scroll_is_owned_and_reset_by_chat_input() {
    let mut chat_input = ChatInput::for_working_directory(".");

    chat_input.set_text("/");
    assert!(chat_input.interaction().is_visible());
    assert!(chat_input.scroll_interaction(
        ScrollCommand::ByPixels(ScrollDelta::vertical(70.0)),
        Size::new(300.0, 100.0),
        Size::new(300.0, 400.0),
    ));
    assert_eq!(
        chat_input.interaction_scroll().offset(),
        Point::new(0.0, 70.0)
    );

    chat_input.activate_selected_interaction();
    assert!(chat_input.interaction().is_model_picker_visible());
    assert_eq!(chat_input.interaction_scroll(), Default::default());
    assert!(chat_input.scroll_interaction(
        ScrollCommand::ByPixels(ScrollDelta::vertical(35.0)),
        Size::new(300.0, 100.0),
        Size::new(300.0, 400.0),
    ));

    chat_input.dismiss_interaction();
    assert!(chat_input.interaction().is_visible());
    assert!(!chat_input.interaction().is_model_picker_visible());
    assert_eq!(chat_input.interaction_scroll(), Default::default());

    chat_input.set_text("");
    assert!(!chat_input.interaction().is_visible());
    assert_eq!(chat_input.interaction_scroll(), Default::default());
}

#[test]
fn interaction_scroll_reveals_content_from_geometry() {
    let mut chat_input = ChatInput::default();

    assert!(chat_input.scroll_interaction(
        ScrollCommand::EnsureVisible(Rect::from_xywh(0.0, 238.0, 300.0, 34.0)),
        Size::new(300.0, 102.0),
        Size::new(300.0, 340.0),
    ));

    assert_eq!(chat_input.interaction_scroll().vertical_offset(), 170.0);
    chat_input.reset_interaction_scroll();
    assert_eq!(chat_input.interaction_scroll(), Default::default());
}
