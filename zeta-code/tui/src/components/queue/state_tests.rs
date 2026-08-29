use super::Queue;
use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputQueueOutcome;

fn queued_input(text: &str) -> crate::components::chat_input::QueuedChatInput {
    let mut input = ChatInput::new();
    input.insert_text(text);
    let ChatInputQueueOutcome::Queued(input) = input.queue_current() else {
        panic!("expected queued input");
    };
    input
}

#[test]
fn queue_preserves_items_until_their_exact_send_finishes() {
    let mut queue = Queue::default();
    queue.push(queued_input("first"));
    queue.push(queued_input("second"));

    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.text, item.sending))
            .collect::<Vec<_>>(),
        [("first", false), ("second", false)]
    );
    assert!(!queue.is_empty());

    let (first_id, submission) = queue.begin_next_send().unwrap();
    assert_eq!(submission.display_text, "first");
    assert_eq!(
        queue
            .view()
            .items
            .iter()
            .map(|item| (item.text, item.sending))
            .collect::<Vec<_>>(),
        [("first", true), ("second", false)]
    );

    assert!(queue.fail_send(first_id));
    assert!(!queue.view().items[0].sending);
    let (retry_id, _) = queue.begin_next_send().unwrap();
    assert_eq!(retry_id, first_id);
    assert!(queue.finish_send(retry_id));
    assert_eq!(queue.view().items[0].text, "second");
}
