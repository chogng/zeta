use super::*;
use zeta_protocol::CommandId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;

#[test]
fn stored_event_round_trip_preserves_history_contract() {
    let thread_id = ThreadId::new("thread_1").unwrap();
    let event = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event_1".into()),
        sequence: 1,
        thread_id: thread_id.clone(),
        recorded_at: Timestamp(42),
        command: Some(ThreadCommandReceipt {
            command_id: CommandId::new("command_1").unwrap(),
            command: ThreadCommand::StartShellTurn {
                command: "pwd".into(),
            },
        }),
        event: ThreadEvent::ThreadCreated {
            session_id: SessionId::new("session_1").unwrap(),
            thread_id: thread_id.clone(),
            title: "Primary".into(),
        },
    };

    let encoded = serde_json::to_string(&event).unwrap();
    let decoded: StoredEvent = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, event);
    assert_eq!(decoded.thread_id(), &thread_id);
}

#[test]
fn supported_schema_range_distinguishes_reads_from_new_writes() {
    assert!(!supports_stored_event_schema_version(0));
    assert!(supports_stored_event_schema_version(
        MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION
    ));
    assert!(supports_stored_event_schema_version(
        CURRENT_STORED_EVENT_SCHEMA_VERSION
    ));
    assert!(!supports_stored_event_schema_version(
        CURRENT_STORED_EVENT_SCHEMA_VERSION + 1
    ));
}
