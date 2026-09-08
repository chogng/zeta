use super::ChatInput;
use super::ChatInputItem;
use super::ChatInputOutcome;
use super::ChatInputQueueOutcome;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn issue_tags_submit_identities_with_the_users_instructions() {
    let mut input = ChatInput::new();
    input.attach_issue(3);
    input.attach_issue(5);
    input.attach_issue(3);
    input.insert_text("fix together");
    assert_eq!(input.text(), "[issue #3] [issue #5] fix together");
    let ChatInputOutcome::Submit(submission) = input.submit_current() else {
        panic!("expected submission");
    };
    assert_eq!(
        submission.input,
        vec![
            ChatInputItem::Issue { number: 3 },
            ChatInputItem::Issue { number: 5 },
            ChatInputItem::Text("fix together".into())
        ]
    );
    assert!(input.is_empty());
}

#[test]
fn typing_an_issue_label_cannot_create_a_reference() {
    let mut input = ChatInput::new();
    input.insert_text("[issue #3]");
    let ChatInputOutcome::Submit(submission) = input.submit_current() else {
        panic!("expected submission");
    };
    assert_eq!(
        submission.input,
        vec![ChatInputItem::Text("[issue #3]".into())]
    );
}

#[test]
fn deleting_an_issue_tag_removes_its_submission_identity() {
    let mut input = ChatInput::new();
    input.attach_issue(3);
    input.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    input.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(input.text(), "");
    assert_eq!(input.submit_current(), ChatInputOutcome::Consumed);
    input.attach_issue(3);
    let ChatInputOutcome::Submit(submission) = input.submit_current() else {
        panic!("expected submission");
    };
    assert_eq!(submission.input, vec![ChatInputItem::Issue { number: 3 }]);
}

#[test]
fn queued_issue_draft_restores_the_atomic_tags() {
    let mut input = ChatInput::new();
    input.attach_issue(5);
    input.insert_text("check regression");
    let ChatInputQueueOutcome::Queued(queued) = input.queue_current() else {
        panic!("expected queued input");
    };
    input.restore_queued(queued).unwrap();
    let ChatInputOutcome::Submit(submission) = input.submit_current() else {
        panic!("expected submission");
    };
    assert_eq!(
        submission.input,
        vec![
            ChatInputItem::Issue { number: 5 },
            ChatInputItem::Text("check regression".into())
        ]
    );
}
