use super::*;
use crate::{CURRENT_SESSION_EVENT_SCHEMA_VERSION, SessionEventId, SessionTimestamp};
use zeta_protocol::{SessionEvent, SessionId};

#[test]
fn validation_rejects_stale_sequence_and_mismatched_identity() {
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let mut batch = SessionEventBatch {
        batch_id: "batch_1".into(),
        session_id: session_id.clone(),
        expected_sequence: 0,
        events: vec![StoredSessionEvent {
            schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
            event_id: SessionEventId("event_1".into()),
            sequence: 1,
            session_id: session_id.clone(),
            recorded_at: SessionTimestamp(1),
            command: None,
            event: SessionEvent::SessionCreated {
                session_id,
                title: "test".into(),
                model: None,
            },
        }],
    };

    assert!(validate_session_append_batch(&batch, 1).is_err());
    batch.events[0].session_id = SessionId::new("other").expect("test ID is non-empty");
    assert!(validate_session_append_batch(&batch, 0).is_err());
}
