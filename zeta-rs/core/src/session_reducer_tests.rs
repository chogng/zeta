use super::*;
use zeta_protocol::{CommandId, SessionThreadStatus};
use zeta_session_store::{SessionEventId, SessionTimestamp};

fn envelope(
    sequence: u64,
    command: Option<SessionCommandReceipt>,
    event: SessionEvent,
) -> StoredSessionEvent {
    StoredSessionEvent {
        schema_version: CURRENT_SESSION_EVENT_SCHEMA_VERSION,
        event_id: SessionEventId(format!("event_{sequence}")),
        sequence,
        session_id: SessionId::new("session_1").expect("test ID is non-empty"),
        recorded_at: SessionTimestamp(u128::from(sequence)),
        command,
        event,
    }
}

fn receipt(id: &str, command: SessionCommand) -> SessionCommandReceipt {
    SessionCommandReceipt {
        command_id: CommandId::new(id).expect("test ID is non-empty"),
        command,
    }
}

#[test]
fn reducer_projects_a_recoverable_thread_creation_saga() {
    let session = reduce_session_event(
        None,
        &envelope(
            1,
            Some(receipt(
                "create-session",
                SessionCommand::Create {
                    title: "task".into(),
                    model: None,
                },
            )),
            SessionEvent::SessionCreated {
                session_id: SessionId::new("session_1").expect("test ID is non-empty"),
                title: "task".into(),
                model: None,
            },
        ),
    )
    .unwrap();
    let session = reduce_session_event(
        Some(session),
        &envelope(
            2,
            Some(receipt(
                "create-thread",
                SessionCommand::CreateThread {
                    title: "root".into(),
                },
            )),
            SessionEvent::ThreadCreationPlanned {
                session_id: SessionId::new("session_1").expect("test ID is non-empty"),
                thread: SessionThread {
                    thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                    origin: ThreadOrigin::Root,
                    status: SessionThreadStatus::Creating,
                },
                title: "root".into(),
            },
        ),
    )
    .unwrap();
    assert_eq!(
        session.threads[0].membership.status,
        SessionThreadStatus::Creating
    );

    let session = reduce_session_event(
        Some(session),
        &envelope(
            3,
            None,
            SessionEvent::ThreadAttached {
                session_id: SessionId::new("session_1").expect("test ID is non-empty"),
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
            },
        ),
    )
    .unwrap();

    assert_eq!(
        session.public_session().threads[0].status,
        SessionThreadStatus::Active
    );
    assert_eq!(session.commands[1].response_sequence, 3);
}

#[test]
fn reducer_rejects_fork_from_an_unknown_parent() {
    let session = reduce_session_event(
        None,
        &envelope(
            1,
            Some(receipt(
                "create-session",
                SessionCommand::Create {
                    title: "task".into(),
                    model: None,
                },
            )),
            SessionEvent::SessionCreated {
                session_id: SessionId::new("session_1").expect("test ID is non-empty"),
                title: "task".into(),
                model: None,
            },
        ),
    )
    .unwrap();
    let result = reduce_session_event(
        Some(session),
        &envelope(
            2,
            Some(receipt(
                "fork",
                SessionCommand::ForkThread {
                    parent_thread_id: ThreadId::new("missing").expect("test ID is non-empty"),
                    title: "branch".into(),
                },
            )),
            SessionEvent::ThreadCreationPlanned {
                session_id: SessionId::new("session_1").expect("test ID is non-empty"),
                thread: SessionThread {
                    thread_id: ThreadId::new("thread_2").expect("test ID is non-empty"),
                    origin: ThreadOrigin::Fork {
                        parent_thread_id: ThreadId::new("missing").expect("test ID is non-empty"),
                        parent_sequence: 1,
                    },
                    status: SessionThreadStatus::Creating,
                },
                title: "branch".into(),
            },
        ),
    );

    assert!(matches!(result, Err(CoreError::NotFound(_))));
}
