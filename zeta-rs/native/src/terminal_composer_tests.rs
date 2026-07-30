use super::TerminalComposer;
use zeta_ui::{TextInputCommand, TextInputCompositionEvent};

#[test]
fn exposes_only_non_empty_commands_for_submission() {
    let mut composer = TerminalComposer::default();

    assert_eq!(composer.command(), None);
    composer.apply(TextInputCommand::Insert("  ".to_string()));
    assert_eq!(composer.command(), None);
    composer.apply(TextInputCommand::Insert("pwd".to_string()));
    assert_eq!(composer.command(), Some("  pwd"));
}

#[test]
fn successful_submission_clears_text_and_composition() {
    let mut composer = TerminalComposer::default();
    composer.apply(TextInputCommand::Insert("echo ".to_string()));
    composer.apply_composition(TextInputCompositionEvent::Commit("你好".to_string()));

    composer.clear_after_submit();

    assert_eq!(composer.input().text(), "");
    assert_eq!(composer.input().composition(), None);
}
