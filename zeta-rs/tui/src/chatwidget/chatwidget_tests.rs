use super::ChatWidget;
use super::ChatWidgetOutcome;
use super::MessageRole;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn submit_records_a_trimmed_user_message() {
    let mut widget = ChatWidget::new();
    widget.insert_text("  explain this  ");

    let outcome = widget.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(outcome, ChatWidgetOutcome::Submit("explain this".into()));
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
