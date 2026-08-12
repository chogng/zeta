use super::*;

#[test]
fn transient_backlog_is_bounded_without_losing_control_messages() {
    let queue = NotificationQueue::default();
    for sequence in 1..=MAX_NOTIFICATION_QUEUE_LEN {
        queue.push(serde_json::json!({
            "jsonrpc":"2.0",
            "method":"session/thread/update",
            "params":{
                "streamCursor":{"streamInstanceId":"stream-1","sequence":sequence},
                "update":{"type":"itemDelta"}
            }
        }));
    }
    queue.push(serde_json::json!({
        "jsonrpc":"2.0",
        "method":"config/changed",
        "params":{"revision":1,"generation":1}
    }));

    let values = queue.drain();
    assert!(values.len() <= MAX_NOTIFICATION_QUEUE_LEN);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["method"], "config/changed");
}

#[test]
fn control_overflow_closes_instead_of_dropping_existing_messages() {
    let queue = NotificationQueue::default();
    for sequence in 0..MAX_NOTIFICATION_QUEUE_LEN {
        queue.push(serde_json::json!({
            "jsonrpc":"2.0",
            "method":"config/changed",
            "params":{"revision":sequence,"generation":sequence}
        }));
    }
    queue.push(serde_json::json!({
        "jsonrpc":"2.0",
        "method":"skills/changed",
        "params":{"generation":1}
    }));

    assert_eq!(queue.len(), MAX_NOTIFICATION_QUEUE_LEN);
    assert!(queue.listener().wait());
    assert_eq!(queue.drain().len(), MAX_NOTIFICATION_QUEUE_LEN);
    assert!(!queue.listener().wait());
}
