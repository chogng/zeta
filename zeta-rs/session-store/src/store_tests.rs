use super::*;
use crate::SessionEventId;
use crate::SessionTimestamp;
use crate::CURRENT_SESSION_EVENT_SCHEMA_VERSION;
use crate::MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionId;

fn stored_session_created(
    session_id: &SessionId,
    schema_version: u32,
    event_id: &str,
    sequence: u64,
) -> StoredSessionEvent {
    StoredSessionEvent {
        schema_version,
        event_id: SessionEventId(event_id.into()),
        sequence,
        session_id: session_id.clone(),
        recorded_at: SessionTimestamp(1),
        command: None,
        event: SessionEvent::SessionCreated {
            session_id: session_id.clone(),
            title: "test".into(),
            model: None,
        },
    }
}

#[test]
fn validation_rejects_stale_sequence_and_mismatched_identity() {
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let mut batch = SessionEventBatch {
        batch_id: "batch_1".into(),
        session_id: session_id.clone(),
        expected_sequence: 0,
        events: vec![stored_session_created(
            &session_id,
            CURRENT_SESSION_EVENT_SCHEMA_VERSION,
            "event_1",
            1,
        )],
    };

    assert!(validate_session_append_batch(&batch, 1).is_err());
    batch.events[0].session_id = SessionId::new("other").expect("test ID is non-empty");
    assert!(validate_session_append_batch(&batch, 0).is_err());
}

#[test]
fn append_requires_current_schema_but_history_accepts_supported_legacy_schema() {
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let event = stored_session_created(
        &session_id,
        MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION,
        "event_1",
        1,
    );
    let batch = SessionEventBatch {
        batch_id: "batch_1".into(),
        session_id: session_id.clone(),
        expected_sequence: 0,
        events: vec![event.clone()],
    };

    assert!(validate_session_append_batch(&batch, 0).is_err());
    assert_eq!(validate_session_history(&session_id, &[event]), Ok(()));
}

#[test]
fn history_rejects_unsupported_schema_gaps_and_duplicate_event_ids() {
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let unsupported = stored_session_created(
        &session_id,
        CURRENT_SESSION_EVENT_SCHEMA_VERSION + 1,
        "event_1",
        1,
    );
    assert!(validate_session_history(&session_id, &[unsupported]).is_err());

    let first = stored_session_created(
        &session_id,
        CURRENT_SESSION_EVENT_SCHEMA_VERSION,
        "event_1",
        1,
    );
    let gap = stored_session_created(
        &session_id,
        CURRENT_SESSION_EVENT_SCHEMA_VERSION,
        "event_2",
        3,
    );
    assert!(validate_session_history(&session_id, &[first.clone(), gap]).is_err());

    let duplicate = stored_session_created(
        &session_id,
        CURRENT_SESSION_EVENT_SCHEMA_VERSION,
        "event_1",
        2,
    );
    assert!(validate_session_history(&session_id, &[first, duplicate]).is_err());
}
