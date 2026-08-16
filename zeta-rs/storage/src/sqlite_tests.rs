use super::SqliteSessionStore;
use super::SqliteThreadStore;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::EventId;
use zeta_history::StoredEvent;
use zeta_history::Timestamp;
use zeta_protocol::CommandId;
use zeta_protocol::SessionCommand;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_session_store::CURRENT_SESSION_EVENT_SCHEMA_VERSION;
use zeta_session_store::MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION;
use zeta_session_store::SessionCommandReceipt;
use zeta_session_store::SessionEventBatch;
use zeta_session_store::SessionEventId;
use zeta_session_store::SessionStore;
use zeta_session_store::SessionStoreError;
use zeta_session_store::SessionTimestamp;
use zeta_session_store::StoredSessionEvent;
use zeta_thread_store::ThreadEventBatch;
use zeta_thread_store::ThreadStore;
use zeta_thread_store::ThreadStoreError;

fn database_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-sqlite-{label}-{}-{}.sqlite3",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn sqlite_stores_share_one_database_and_recover_typed_events() {
    let path = database_path("recovery");
    let session_id = SessionId::new("session_1").unwrap();
    let thread_id = ThreadId::new("thread_1").unwrap();
    let session_event = StoredSessionEvent {
        schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
        event_id: SessionEventId("session-event-1".into()),
        sequence: 1,
        session_id: session_id.clone(),
        recorded_at: SessionTimestamp(1),
        command: Some(SessionCommandReceipt {
            command_id: CommandId::new("create-session").unwrap(),
            command: SessionCommand::Create {
                title: "Task".into(),
                model: None,
                workspace: None,
            },
        }),
        event: SessionEvent::SessionCreated {
            session_id: session_id.clone(),
            title: "Task".into(),
            model: None,
            workspace: None,
        },
    };
    let thread_event = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("thread-event-1".into()),
        sequence: 1,
        thread_id: thread_id.clone(),
        recorded_at: Timestamp(2),
        command: None,
        event: ThreadEvent::ThreadCreated {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            title: "Primary".into(),
        },
    };

    SqliteSessionStore::open(&path)
        .unwrap()
        .append_batch(&SessionEventBatch {
            batch_id: "session-batch-1".into(),
            session_id: session_id.clone(),
            expected_sequence: 0,
            events: vec![session_event.clone()],
        })
        .unwrap();
    SqliteThreadStore::open(&path)
        .unwrap()
        .append_batch(&ThreadEventBatch {
            batch_id: "thread-batch-1".into(),
            thread_id: thread_id.clone(),
            expected_sequence: 0,
            events: vec![thread_event.clone()],
        })
        .unwrap();

    assert_eq!(
        SqliteSessionStore::open(&path)
            .unwrap()
            .load(&session_id)
            .unwrap(),
        vec![session_event]
    );
    assert_eq!(
        SqliteThreadStore::open(&path)
            .unwrap()
            .load(&thread_id)
            .unwrap(),
        vec![thread_event]
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn sqlite_thread_append_is_atomic_and_sequence_checked() {
    let path = database_path("sequence");
    let thread_id = ThreadId::new("thread_1").unwrap();
    let event = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event-1".into()),
        sequence: 1,
        thread_id: thread_id.clone(),
        recorded_at: Timestamp(1),
        command: None,
        event: ThreadEvent::ThreadCreated {
            session_id: SessionId::new("session_1").unwrap(),
            thread_id: thread_id.clone(),
            title: "Primary".into(),
        },
    };
    let store = SqliteThreadStore::open(&path).unwrap();
    store
        .append_batch(&ThreadEventBatch {
            batch_id: "batch-1".into(),
            thread_id: thread_id.clone(),
            expected_sequence: 0,
            events: vec![event.clone()],
        })
        .unwrap();
    let stale = store.append_batch(&ThreadEventBatch {
        batch_id: "batch-2".into(),
        thread_id: thread_id.clone(),
        expected_sequence: 0,
        events: vec![event],
    });

    assert!(matches!(
        stale,
        Err(ThreadStoreError::SequenceConflict {
            expected: 0,
            actual: 1
        })
    ));
    assert_eq!(store.load(&thread_id).unwrap().len(), 1);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn sqlite_thread_recovery_rejects_metadata_mismatch_and_accepts_legacy_schema() {
    let path = database_path("legacy-schema");
    let thread_id = ThreadId::new("thread_1").unwrap();
    let mut event = StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId("event-1".into()),
        sequence: 1,
        thread_id: thread_id.clone(),
        recorded_at: Timestamp(1),
        command: None,
        event: ThreadEvent::ThreadCreated {
            session_id: SessionId::new("session_1").unwrap(),
            thread_id: thread_id.clone(),
            title: "Primary".into(),
        },
    };
    let store = SqliteThreadStore::open(&path).unwrap();
    store
        .append_batch(&ThreadEventBatch {
            batch_id: "batch-1".into(),
            thread_id: thread_id.clone(),
            expected_sequence: 0,
            events: vec![event.clone()],
        })
        .unwrap();
    drop(store);

    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE thread_events SET schema_version = ?1 WHERE thread_id = ?2 AND sequence = 1",
            rusqlite::params![
                zeta_history::MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION,
                thread_id.as_str()
            ],
        )
        .unwrap();
    assert!(matches!(
        SqliteThreadStore::open(&path)
            .unwrap()
            .load(&thread_id),
        Err(ThreadStoreError::Storage(message))
            if message.contains("metadata disagrees")
    ));

    event.schema_version = zeta_history::MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION;
    connection
        .execute(
            "UPDATE thread_events SET envelope_json = ?1 WHERE thread_id = ?2 AND sequence = 1",
            rusqlite::params![serde_json::to_string(&event).unwrap(), thread_id.as_str()],
        )
        .unwrap();

    assert_eq!(
        SqliteThreadStore::open(&path)
            .unwrap()
            .load(&thread_id)
            .unwrap(),
        vec![event.clone()]
    );
    connection
        .execute(
            "UPDATE thread_streams SET current_sequence = 2 WHERE thread_id = ?1",
            [thread_id.as_str()],
        )
        .unwrap();
    assert!(matches!(
        SqliteThreadStore::open(&path)
            .unwrap()
            .load(&thread_id),
        Err(ThreadStoreError::Storage(message))
            if message.contains("durable event tail")
    ));
    drop(connection);
    fs::remove_file(path).unwrap();
}

#[test]
fn sqlite_session_recovery_rejects_corruption_and_accepts_legacy_schema() {
    let path = database_path("session-legacy-schema");
    let session_id = SessionId::new("session_1").unwrap();
    let mut event = StoredSessionEvent {
        schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
        event_id: SessionEventId("session-event-1".into()),
        sequence: 1,
        session_id: session_id.clone(),
        recorded_at: SessionTimestamp(1),
        command: Some(SessionCommandReceipt {
            command_id: CommandId::new("create-session").unwrap(),
            command: SessionCommand::Create {
                title: "Task".into(),
                model: None,
                workspace: None,
            },
        }),
        event: SessionEvent::SessionCreated {
            session_id: session_id.clone(),
            title: "Task".into(),
            model: None,
            workspace: None,
        },
    };
    let store = SqliteSessionStore::open(&path).unwrap();
    store
        .append_batch(&SessionEventBatch {
            batch_id: "session-batch-1".into(),
            session_id: session_id.clone(),
            expected_sequence: 0,
            events: vec![event.clone()],
        })
        .unwrap();
    drop(store);

    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE session_events SET schema_version = ?1 WHERE session_id = ?2 AND sequence = 1",
            rusqlite::params![
                MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION,
                session_id.as_str()
            ],
        )
        .unwrap();
    assert!(matches!(
        SqliteSessionStore::open(&path).unwrap().load(&session_id),
        Err(SessionStoreError::Storage(message)) if message.contains("metadata disagrees")
    ));

    event.schema_version = MINIMUM_SUPPORTED_SESSION_EVENT_SCHEMA_VERSION;
    connection
        .execute(
            "UPDATE session_events SET envelope_json = ?1 WHERE session_id = ?2 AND sequence = 1",
            rusqlite::params![serde_json::to_string(&event).unwrap(), session_id.as_str()],
        )
        .unwrap();
    assert_eq!(
        SqliteSessionStore::open(&path)
            .unwrap()
            .load(&session_id)
            .unwrap(),
        vec![event]
    );

    connection
        .execute(
            "UPDATE session_streams SET current_sequence = 2 WHERE session_id = ?1",
            [session_id.as_str()],
        )
        .unwrap();
    assert!(matches!(
        SqliteSessionStore::open(&path).unwrap().load(&session_id),
        Err(SessionStoreError::Storage(message)) if message.contains("durable event tail")
    ));
    drop(connection);
    fs::remove_file(path).unwrap();
}

#[cfg(unix)]
#[test]
fn sqlite_authority_database_is_private_to_the_host_user() {
    use std::os::unix::fs::PermissionsExt;

    let path = database_path("permissions");
    let store = SqliteSessionStore::open(&path).unwrap();

    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);

    drop(store);
    fs::remove_file(path).unwrap();
}
