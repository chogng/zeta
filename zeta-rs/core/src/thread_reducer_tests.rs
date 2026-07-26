use super::*;
use zeta_protocol::CommandId;
use zeta_protocol::ItemId;
use zeta_protocol::StableTurnErrorCode;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::UserInput;
use zeta_thread_store::{EventId, ThreadCommandReceipt, Timestamp};

fn envelope(sequence: u64, event: ThreadEvent) -> StoredEvent {
    let command = match &event {
        ThreadEvent::TurnAccepted { .. } => Some(ThreadCommandReceipt {
            command_id: CommandId::new(format!("command_{sequence}"))
                .expect("test ID is non-empty"),
            command: ThreadCommand::StartTurn {
                input: vec![UserInput::Text {
                    text: "hello".into(),
                }],
            },
        }),
        _ => None,
    };
    StoredEvent {
        schema_version: CURRENT_STORED_EVENT_SCHEMA_VERSION,
        event_id: EventId(format!("event_{sequence}")),
        sequence,
        thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
        recorded_at: Timestamp(u128::from(sequence)),
        command,
        event,
    }
}

#[test]
fn reducer_rebuilds_a_failed_turn_with_stable_error_details() {
    let thread = reduce_thread_event(
        None,
        &envelope(
            1,
            ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1")
                    .expect("test ID is non-empty"),
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                title: "test".into(),
            },
        ),
    )
    .unwrap();
    let thread = reduce_thread_event(
        Some(thread),
        &envelope(
            2,
            ThreadEvent::TurnAccepted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        ),
    )
    .unwrap();
    let thread = reduce_thread_event(
        Some(thread),
        &envelope(
            3,
            ThreadEvent::TurnStarted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        ),
    )
    .unwrap();
    let thread = reduce_thread_event(
        Some(thread),
        &envelope(
            4,
            ThreadEvent::TurnFailed {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                error: StableTurnError::model_invocation_failed(),
            },
        ),
    )
    .unwrap();

    assert_eq!(thread.turns[0].status, TurnStatus::Failed);
    assert_eq!(
        thread.turns[0].failure.as_ref().unwrap().code,
        StableTurnErrorCode::ModelInvocationFailed
    );
}

#[test]
fn reducer_rejects_sequence_gaps_and_illegal_transitions() {
    let thread = reduce_thread_event(
        None,
        &envelope(
            1,
            ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1")
                    .expect("test ID is non-empty"),
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                title: "test".into(),
            },
        ),
    )
    .unwrap();
    assert!(
        reduce_thread_event(
            Some(thread.clone()),
            &envelope(
                3,
                ThreadEvent::TurnAccepted {
                    thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty")
                }
            )
        )
        .is_err()
    );

    let thread = reduce_thread_event(
        Some(thread),
        &envelope(
            2,
            ThreadEvent::TurnAccepted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        ),
    )
    .unwrap();
    assert!(
        reduce_thread_event(
            Some(thread),
            &envelope(
                3,
                ThreadEvent::TurnCompleted {
                    thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty")
                }
            )
        )
        .is_err()
    );
}

#[test]
fn reducer_rebuilds_typed_command_receipt_and_all_durable_item_kinds() {
    let mut accepted = envelope(
        2,
        ThreadEvent::TurnAccepted {
            thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
            turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
        },
    );
    accepted.command = Some(ThreadCommandReceipt {
        command_id: CommandId::new("command_1").expect("test ID is non-empty"),
        command: ThreadCommand::StartTurn {
            input: vec![UserInput::Text {
                text: "hello".into(),
            }],
        },
    });
    let events = [
        envelope(
            1,
            ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1")
                    .expect("test ID is non-empty"),
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                title: "test".into(),
            },
        ),
        accepted,
        envelope(
            3,
            ThreadEvent::ItemCompleted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                item: ThreadItem::UserMessage {
                    item_id: ItemId::new("item_1").expect("test ID is non-empty"),
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                    text: "hello".into(),
                },
            },
        ),
        envelope(
            4,
            ThreadEvent::TurnStarted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        ),
        envelope(
            5,
            ThreadEvent::ItemCompleted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                item: ThreadItem::ToolCall {
                    item_id: ItemId::new("item_2").expect("test ID is non-empty"),
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                    tool_call_id: ToolCallId::new("tool_1").expect("test ID is non-empty"),
                    name: ToolName::new("search").expect("test tool name is valid"),
                    arguments_json: r#"{"query":"zeta"}"#.into(),
                },
            },
        ),
        envelope(
            6,
            ThreadEvent::ItemCompleted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                item: ThreadItem::ToolResult {
                    item_id: ItemId::new("item_3").expect("test ID is non-empty"),
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                    tool_call_id: ToolCallId::new("tool_1").expect("test ID is non-empty"),
                    text: "result".into(),
                    is_error: false,
                },
            },
        ),
        envelope(
            7,
            ThreadEvent::ItemCompleted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                item: ThreadItem::AgentMessage {
                    item_id: ItemId::new("item_4").expect("test ID is non-empty"),
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                    text: "answer".into(),
                },
            },
        ),
        envelope(
            8,
            ThreadEvent::TurnCompleted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        ),
    ];
    let snapshot = events
        .iter()
        .try_fold(None, |snapshot, event| {
            reduce_thread_event(snapshot, event).map(Some)
        })
        .unwrap()
        .unwrap();

    assert_eq!(snapshot.items.len(), 4);
    assert_eq!(snapshot.commands.len(), 1);
    assert_eq!(snapshot.commands[0].response_sequence, 4);
    assert!(matches!(
        &snapshot.commands[0].result,
        ThreadCommandResult::TurnAccepted { turn_id }
            if turn_id == &TurnId::new("turn_1").expect("test ID is non-empty")
    ));
    assert_eq!(snapshot.turns[0].status, TurnStatus::Completed);
}

#[test]
fn reducer_rejects_a_tool_result_without_its_tool_call() {
    let thread = reduce_thread_event(
        None,
        &envelope(
            1,
            ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1")
                    .expect("test ID is non-empty"),
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                title: "test".into(),
            },
        ),
    )
    .unwrap();
    let thread = reduce_thread_event(
        Some(thread),
        &envelope(
            2,
            ThreadEvent::TurnAccepted {
                thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
            },
        ),
    )
    .unwrap();

    assert!(
        reduce_thread_event(
            Some(thread),
            &envelope(
                3,
                ThreadEvent::ItemCompleted {
                    thread_id: ThreadId::new("thread_1").expect("test ID is non-empty"),
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                    item: ThreadItem::ToolResult {
                        item_id: ItemId::new("item_1").expect("test ID is non-empty"),
                        turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                        tool_call_id: ToolCallId::new("missing").expect("test ID is non-empty"),
                        text: "result".into(),
                        is_error: false,
                    },
                },
            ),
        )
        .is_err()
    );
}
