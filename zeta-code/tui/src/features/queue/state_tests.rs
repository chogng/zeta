use super::Queue;
use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputQueueOutcome;

fn queued_input(text: &str) -> crate::components::chat_input::QueuedChatInput {
    let mut input = ChatInput::new();
    input.insert_text(text);
    let ChatInputQueueOutcome::Queued(input) = input.queue_current(None) else {
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
            .map(|item| (item.position, item.text, item.sending))
            .collect::<Vec<_>>(),
        [(1, "first", false), (2, "second", false)]
    );
    assert!(!queue.is_empty());

    let (first_id, submission) = queue.begin_next_send().unwrap();
    assert_eq!(submission.display_text, "first");
    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.position, item.text, item.sending))
            .collect::<Vec<_>>(),
        [(1, "first", true), (2, "second", false)]
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
fn restore_preserves_a_nonempty_draft_and_restores_by_stable_identity() {
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
    assert_eq!(queue.view().items[0].id, first);
}
