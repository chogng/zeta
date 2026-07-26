use super::*;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_protocol::{CommandId, SessionCommand, SessionEvent};
use zeta_session_store::{
    CURRENT_SESSION_EVENT_SCHEMA_VERSION, SessionCommandReceipt, SessionEventId, SessionTimestamp,
};

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-session-rollout-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn session_rollouts_keep_independent_sequences_and_typed_receipts() {
    let root = temp_root();
    let store = SessionRolloutStore::open(&root).unwrap();
    for index in 1..=2 {
        let session_id = SessionId::new(format!("session_{index}")).expect("test ID is non-empty");
        let event = StoredSessionEvent {
            schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
            event_id: SessionEventId(format!("event_{index}")),
            sequence: 1,
            session_id: session_id.clone(),
            recorded_at: SessionTimestamp(index),
            command: Some(SessionCommandReceipt {
                command_id: CommandId::new(format!("command_{index}"))
                    .expect("test ID is non-empty"),
                command: SessionCommand::Create {
                    title: format!("task {index}"),
                },
            }),
            event: SessionEvent::SessionCreated {
                session_id: session_id.clone(),
                title: format!("task {index}"),
            },
        };
        store
            .append_batch(&SessionEventBatch {
                batch_id: format!("batch_{index}"),
                session_id: session_id.clone(),
                expected_sequence: 0,
                events: vec![event.clone()],
            })
            .unwrap();
        assert_eq!(store.load(&session_id).unwrap(), vec![event]);
    }
    assert_eq!(store.list_session_ids().unwrap().len(), 2);
    fs::remove_dir_all(root).unwrap();
}
