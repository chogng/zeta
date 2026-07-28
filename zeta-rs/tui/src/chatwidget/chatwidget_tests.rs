use super::ChatWidget;
use super::ChatWidgetOutcome;
use super::MessageRole;
use crate::toppane::ComposerInput;
use crate::toppane::SlashCommand;
use crate::toppane::SlashCommandInvocation;
use crate::toppane::SlashCommandItem;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn submit_records_a_trimmed_user_message() {
    let mut widget = ChatWidget::new();
    widget.insert_text("  explain this  ");

    let outcome = widget.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let ChatWidgetOutcome::Submit(submission) = outcome else {
        panic!("expected submission");
    };
    assert_eq!(submission.display_text, "explain this");
    assert_eq!(
        submission.input,
        vec![ComposerInput::Text("explain this".into())]
    );
    assert_eq!(widget.draft(), "");
    assert_eq!(widget.messages().len(), 1);
    assert_eq!(widget.messages()[0].role, MessageRole::User);
    assert_eq!(widget.messages()[0].text, "explain this");
}

#[test]
fn blank_submit_is_consumed_without_recording_a_message() {
    let mut widget = ChatWidget::new();
    widget.insert_text("   ");

    let outcome = widget.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(outcome, ChatWidgetOutcome::Consumed);
    assert!(widget.messages().is_empty());
}

#[test]
fn global_control_key_is_returned_to_the_app() {
    let mut widget = ChatWidget::new();

    let outcome = widget.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(outcome, ChatWidgetOutcome::Unhandled);
}

#[test]
fn local_slash_command_is_not_recorded_as_a_user_message() {
    let mut widget = ChatWidget::new();
    widget.insert_text("/quit");

    let outcome = widget.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        outcome,
        ChatWidgetOutcome::Command(SlashCommandInvocation {
            command: SlashCommandItem::Builtin(SlashCommand::Quit),
            display_arguments: String::new(),
            arguments: Vec::new(),
        })
    );
    assert_eq!(widget.draft(), "");
    assert!(widget.messages().is_empty());
}
