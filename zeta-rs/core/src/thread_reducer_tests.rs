use super::*;
use zeta_history::CURRENT_STORED_EVENT_SCHEMA_VERSION;
use zeta_history::EventId;
use zeta_history::ThreadCommandReceipt;
use zeta_history::Timestamp;
use zeta_protocol::CommandId;
use zeta_protocol::ContextCheckpointId;
use zeta_protocol::ContextCheckpointVerification;
use zeta_protocol::ItemId;
use zeta_protocol::StableTurnErrorCode;
use zeta_protocol::ThreadCommand;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::UserInput;

fn envelope(sequence: u64, event: ThreadEvent) -> StoredEvent {
    let command = match &event {
        ThreadEvent::TurnAccepted { .. } => Some(ThreadCommandReceipt {
            command_id: CommandId::new(format!("command_{sequence}"))
                .expect("test ID is non-empty"),
            command: ThreadCommand::StartTurn {
                model: None,
                activated_skills: Vec::new(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
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
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                model: None,
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
fn reducer_replays_schema_v1_turns_with_legacy_policy_and_no_skills() {
    let thread = reduce_thread_event(
        None,
        &envelope(
            1,
            ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1").unwrap(),
                thread_id: ThreadId::new("thread_1").unwrap(),
                title: "legacy".into(),
            },
        ),
    )
    .unwrap();
    let event = serde_json::from_value::<ThreadEvent>(serde_json::json!({
        "type": "turnAccepted",
        "threadId": "thread_1",
        "turnId": "turn_1"
    }))
    .unwrap();
    let mut accepted = envelope(2, event);
    accepted.schema_version = 1;

    let rebuilt = reduce_thread_event(Some(thread), &accepted).unwrap();

    assert_eq!(
        rebuilt.turns[0].policy_revision,
        "legacy-unversioned-policy"
    );
    assert!(rebuilt.turns[0].activated_skills.is_empty());
}

#[test]
fn reducer_verifies_and_rebuilds_a_context_checkpoint() {
    let events = [
        envelope(
            1,
            ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1").unwrap(),
                thread_id: ThreadId::new("thread_1").unwrap(),
                title: "test".into(),
            },
        ),
        envelope(
            2,
            ThreadEvent::TurnAccepted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                model: None,
            },
        ),
        envelope(
            3,
            ThreadEvent::ItemCompleted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
                item: ThreadItem::UserMessage {
                    item_id: ItemId::new("item_1").unwrap(),
                    turn_id: TurnId::new("turn_1").unwrap(),
                    text: "hello".into(),
                },
            },
        ),
        envelope(
            4,
            ThreadEvent::TurnStarted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
            },
        ),
        envelope(
            5,
            ThreadEvent::ItemCompleted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
                item: ThreadItem::AgentMessage {
                    item_id: ItemId::new("item_2").unwrap(),
                    turn_id: TurnId::new("turn_1").unwrap(),
                    text: "answer".into(),
                },
            },
        ),
        envelope(
            6,
            ThreadEvent::TurnCompleted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
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
    let covered = ContextSourceRange {
        start_sequence: 1,
        end_sequence: snapshot.sequence,
    };
    let checkpoint = ContextCheckpoint {
        checkpoint_id: ContextCheckpointId::new("checkpoint_1").unwrap(),
        source_thread_id: snapshot.thread_id.clone(),
        covered,
        referenced_items: snapshot
            .items
            .iter()
            .map(|item| item.item_id().clone())
            .collect(),
        source_digest: snapshot.context_source_digest(covered).unwrap(),
        summary: "durable summary".into(),
        schema_revision: "context-checkpoint-v1".into(),
        prompt_revision: "compaction-v2".into(),
        context_policy_revision: "context-policy-v1".into(),
        generator_model: None,
        created_at_unix_ms: 7,
        verification: ContextCheckpointVerification::Verified,
    };

    let rebuilt = reduce_thread_event(
        Some(snapshot.clone()),
        &envelope(
            7,
            ThreadEvent::ContextCheckpointCommitted {
                thread_id: snapshot.thread_id.clone(),
                checkpoint: checkpoint.clone(),
            },
        ),
    )
    .unwrap();
    assert_eq!(rebuilt.context_checkpoints, vec![checkpoint.clone()]);

    let mut corrupt = checkpoint;
    corrupt.summary = "rewritten summary".into();
    corrupt.source_digest = ContextSourceDigest::new(format!("sha256:{}", "0".repeat(64))).unwrap();
    assert!(
        reduce_thread_event(
            Some(snapshot.clone()),
            &envelope(
                7,
                ThreadEvent::ContextCheckpointCommitted {
                    thread_id: snapshot.thread_id,
                    checkpoint: corrupt,
                },
            ),
        )
        .is_err()
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
                    turn_id: TurnId::new("turn_1").expect("test ID is non-empty"),
                    policy_revision: "test-policy-v1".into(),
                    approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                    activated_skills: Vec::new(),
                    model: None
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
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                model: None,
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
            policy_revision: "test-policy-v1".into(),
            approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
            activated_skills: Vec::new(),
            model: None,
        },
    );
    accepted.command = Some(ThreadCommandReceipt {
        command_id: CommandId::new("command_1").expect("test ID is non-empty"),
        command: ThreadCommand::StartTurn {
            model: None,
            activated_skills: Vec::new(),
            approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
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
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                model: None,
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

#[test]
fn reducer_rejects_unsafe_or_rebound_tool_escalation() {
    let thread = started_sandboxed_tool_snapshot();
    let denial_output = zeta_protocol::ProcessExecutionOutput::from_captured_streams(
        zeta_protocol::ProcessExitStatus::Code(1),
        "",
        "operation not permitted",
    );
    let unsafe_escalation = ThreadEvent::ToolExecutionEscalated {
        thread_id: ThreadId::new("thread_1").unwrap(),
        turn_id: TurnId::new("turn_1").unwrap(),
        tool_call_id: ToolCallId::new("tool_1").unwrap(),
        action_digest: "action-1".into(),
        policy_revision: "policy-1".into(),
        denial: zeta_protocol::SandboxDenialOutput::may_have_side_effects(
            "write denied after process start",
            denial_output.clone(),
        ),
        authority: zeta_protocol::ToolExecutionAuthority::AutoReviewed {
            assessment_id: "assessment-1".into(),
        },
    };
    assert!(reduce_thread_event(Some(thread.clone()), &envelope(6, unsafe_escalation)).is_err());

    let rebound_escalation = ThreadEvent::ToolExecutionEscalated {
        thread_id: ThreadId::new("thread_1").unwrap(),
        turn_id: TurnId::new("turn_1").unwrap(),
        tool_call_id: ToolCallId::new("tool_1").unwrap(),
        action_digest: "another-action".into(),
        policy_revision: "policy-1".into(),
        denial: zeta_protocol::SandboxDenialOutput::safe_to_retry(
            "sandbox setup failed",
            denial_output,
        ),
        authority: zeta_protocol::ToolExecutionAuthority::AutoReviewed {
            assessment_id: "assessment-1".into(),
        },
    };
    assert!(reduce_thread_event(Some(thread.clone()), &envelope(6, rebound_escalation)).is_err());

    let unbound_approval_escalation = ThreadEvent::ToolExecutionEscalated {
        thread_id: ThreadId::new("thread_1").unwrap(),
        turn_id: TurnId::new("turn_1").unwrap(),
        tool_call_id: ToolCallId::new("tool_1").unwrap(),
        action_digest: "action-1".into(),
        policy_revision: "policy-1".into(),
        denial: zeta_protocol::SandboxDenialOutput::safe_to_retry(
            "sandbox setup failed",
            zeta_protocol::ProcessExecutionOutput::from_captured_streams(
                zeta_protocol::ProcessExitStatus::Code(1),
                "",
                "operation not permitted",
            ),
        ),
        authority: zeta_protocol::ToolExecutionAuthority::ApprovedOnce {
            request_id: zeta_protocol::RequestId::new("missing-approval").unwrap(),
        },
    };
    assert!(reduce_thread_event(Some(thread), &envelope(6, unbound_approval_escalation)).is_err());
}

fn started_sandboxed_tool_snapshot() -> ThreadSnapshot {
    let events = [
        envelope(
            1,
            ThreadEvent::ThreadCreated {
                session_id: zeta_protocol::SessionId::new("session_1").unwrap(),
                thread_id: ThreadId::new("thread_1").unwrap(),
                title: "test".into(),
            },
        ),
        envelope(
            2,
            ThreadEvent::TurnAccepted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                activated_skills: Vec::new(),
                model: None,
            },
        ),
        envelope(
            3,
            ThreadEvent::TurnStarted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
            },
        ),
        envelope(
            4,
            ThreadEvent::ItemCompleted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
                item: ThreadItem::ToolCall {
                    item_id: ItemId::new("item_1").unwrap(),
                    turn_id: TurnId::new("turn_1").unwrap(),
                    tool_call_id: ToolCallId::new("tool_1").unwrap(),
                    name: ToolName::new("shell-command").unwrap(),
                    arguments_json: "{}".into(),
                },
            },
        ),
        envelope(
            5,
            ThreadEvent::ToolExecutionStarted {
                thread_id: ThreadId::new("thread_1").unwrap(),
                turn_id: TurnId::new("turn_1").unwrap(),
                tool_call_id: ToolCallId::new("tool_1").unwrap(),
                action_digest: "action-1".into(),
                policy_revision: "policy-1".into(),
                authority: zeta_protocol::ToolExecutionAuthority::Sandboxed,
            },
        ),
    ];
    events
        .iter()
        .try_fold(None, |snapshot, event| {
            reduce_thread_event(snapshot, event).map(Some)
        })
        .unwrap()
        .unwrap()
}
