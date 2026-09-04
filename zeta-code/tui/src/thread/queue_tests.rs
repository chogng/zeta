use super::Queue;
use super::QueueKeyOutcome;
use crate::thread::composer::ChatInput;
use crate::thread::composer::ChatInputQueueOutcome;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

fn queued_input(text: &str) -> crate::thread::composer::QueuedChatInput {
    let mut input = ChatInput::new();
    input.insert_text(text);
    let ChatInputQueueOutcome::Queued(input) = input.queue_current() else {
        panic!("expected queued input");
    };
    input
}

#[test]
fn queue_preserves_stable_identity_and_derives_display_positions() {
    let mut queue = Queue::default();
    queue.push(queued_input("first"));
    queue.push(queued_input("second"));

    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.position, item.text, item.sending, item.editing))
            .collect::<Vec<_>>(),
        [(1, "first", false, false), (2, "second", false, false)]
    );
    assert!(!queue.is_empty());

    let (first_id, submission) = queue.begin_next_send().unwrap();
    assert_eq!(submission.display_text, "first");
    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.position, item.text, item.sending, item.editing))
            .collect::<Vec<_>>(),
        [(1, "first", true, false), (2, "second", false, false)]
    );

    assert!(queue.fail_send(first_id));
    assert!(!queue.view().items[0].sending);
    let (retry_id, _) = queue.begin_next_send().unwrap();
    assert_eq!(retry_id, first_id);
    assert!(queue.finish_send(retry_id));
    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.position, item.text))
            .collect::<Vec<_>>(),
        [(1, "second")]
    );
}

#[test]
fn restore_preserves_position_while_the_message_is_edited() {
    let mut queue = Queue::default();
    let first = queue.push(queued_input("first"));
    let second = queue.push(queued_input("second"));
    let mut input = ChatInput::new();
    input.insert_text("draft");

    assert!(queue.restore(first, &mut input).is_err());
    assert_eq!(input.text(), "draft");
    assert_eq!(queue.view().items.len(), 2);

    input = ChatInput::new();
    queue.restore(second, &mut input).unwrap();
    assert_eq!(input.text(), "second");
    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.id, item.position, item.editing))
            .collect::<Vec<_>>(),
        [(first, 1, false), (second, 2, true)]
    );

    input.insert_text(" updated");
    let ChatInputQueueOutcome::Queued(updated) = input.queue_current() else {
        panic!("expected edited Queue content");
    };
    assert_eq!(queue.push(updated), second);
    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.id, item.position, item.text, item.editing))
            .collect::<Vec<_>>(),
        [
            (first, 1, "first", false),
            (second, 2, "second updated", false)
        ]
    );
}

#[test]
fn focused_queue_supports_selection_reordering_and_actions() {
    let mut queue = Queue::default();
    let first = queue.push(queued_input("first"));
    let second = queue.push(queued_input("second"));

    assert!(queue.focus_latest());
    assert_eq!(queue.view().selected, Some(second));
    assert_eq!(
        queue.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        QueueKeyOutcome::Consumed
    );
    assert_eq!(queue.view().selected, Some(first));
    assert_eq!(
        queue.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL)),
        QueueKeyOutcome::Consumed
    );
    assert_eq!(queue.view().items[1].id, first);
    assert_eq!(queue.view().selected, Some(first));
    assert_eq!(
        queue.handle_key(KeyEvent {
            kind: KeyEventKind::Release,
            ..KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        }),
        QueueKeyOutcome::Consumed
    );
    assert_eq!(
        queue.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        QueueKeyOutcome::Restore(first)
    );
}

#[test]
fn down_after_the_last_message_returns_focus_to_the_composer() {
    let mut queue = Queue::default();
    queue.push(queued_input("only"));

    assert!(queue.focus_latest());
    assert_eq!(
        queue.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        QueueKeyOutcome::Consumed
    );
    assert!(!queue.focused());
}

#[test]
fn deleting_a_selected_message_keeps_the_nearest_message_selected() {
    let mut queue = Queue::default();
    let first = queue.push(queued_input("first"));
    let second = queue.push(queued_input("second"));
    let third = queue.push(queued_input("third"));
    queue.focus_latest();
    queue.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(queue.view().selected, Some(second));

    queue.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        [first, third]
    );
    assert_eq!(queue.view().selected, Some(third));
}
