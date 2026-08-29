use super::Queue;

#[test]
fn replacing_queue_items_updates_the_read_only_view() {
    let mut queue = Queue::default();
    queue.replace(vec!["first".into(), "second".into()]);

    assert_eq!(queue.view().items, ["first", "second"]);
    assert!(!queue.is_empty());

    queue.replace(Vec::new());
    assert!(queue.is_empty());
}
