use super::*;
use zeta_protocol::ItemDelta;
use zeta_protocol::ItemId;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::SessionId;
use zeta_protocol::StreamCursor;
use zeta_protocol::StreamInstanceId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolOutputStream;
use zeta_protocol::TurnId;

#[test]
fn snapshot_preserves_items_and_turn_plans_in_order() {
    let snapshot = ThreadTranscriptSnapshot::from_thread(&Thread {
        session_id: session_id(),
        thread_id: thread_id(),
        parent_thread_id: None,
        forked_from_id: None,
        title: "Thread".into(),
        status: ThreadStatus::Active,
        sequence: 7,
        usage: ModelUsageSummary::default(),
        reference_cost: zeta_protocol::ModelReferenceCostSummary::default(),
        goal: None,
        turns: vec![zeta_protocol::Turn {
            turn_id: turn_id(),
            status: zeta_protocol::TurnStatus::Completed,
            kind: zeta_protocol::TurnKind::Coding,
            instructions: None,
            model: None,
            tool_profile: None,
            tool_mode: zeta_protocol::ToolMode::Direct,
            approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
            usage: ModelUsageSummary::default(),
            context_usage: None,
            items: vec![agent_item("item-1", "done")],
            plan: Some(zeta_protocol::PlanUpdate {
                explanation: Some("why".into()),
                steps: Vec::new(),
            }),
            pending_interaction: None,
            error: None,
        }],
    });

    assert_eq!(snapshot.durable_sequence, 7);
    assert_eq!(snapshot.entries.len(), 2);
    assert_eq!(snapshot.entries[0].entry_id(), "item:item-1");
    assert_eq!(snapshot.entries[1].entry_id(), "turn-plan:turn-1");
}

#[test]
fn deltas_emit_complete_upserts_and_committed_item_replaces_transient() {
    let mut accumulator = TranscriptAccumulator::new(session_id(), thread_id());
    let first = apply(
        &mut accumulator,
        transient(
            1,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-1"),
                delta: ItemDelta::AgentMessage { text: "hel".into() },
            },
        ),
    );
    let second = apply(
        &mut accumulator,
        transient(
            2,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-1"),
                delta: ItemDelta::AgentMessage { text: "lo".into() },
            },
        ),
    );
    let committed = apply(
        &mut accumulator,
        durable(
            1,
            ThreadEvent::ItemCompleted {
                thread_id: thread_id(),
                turn_id: turn_id(),
                item: agent_item("item-1", "hello"),
            },
        ),
    );

    assert_eq!(entry_text(&first), "hel");
    assert_eq!(entry_text(&second), "hello");
    assert_eq!(entry_text(&committed), "hello");
    assert_eq!(revision(&first), 1);
    assert_eq!(revision(&second), 2);
    assert_eq!(revision(&committed), 3);
    let TranscriptApplyResult::Applied(committed) = committed else {
        unreachable!()
    };
    assert!(matches!(
        &committed.changes[0],
        ThreadTranscriptChange::Upsert {
            entry: ThreadTranscriptEntry::Item {
                transient: false,
                ..
            }
        }
    ));
}

#[test]
fn scope_mismatch_and_duplicate_cursor_are_ignored() {
    let mut accumulator = TranscriptAccumulator::new(session_id(), thread_id());
    let wrong = ThreadUpdateEnvelope {
        session_id: SessionId::new("other").unwrap(),
        ..transient(
            1,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-1"),
                delta: ItemDelta::Reasoning { text: "x".into() },
            },
        )
    };
    assert_eq!(accumulator.apply(&wrong), TranscriptApplyResult::Ignored);

    let update = transient(
        1,
        ThreadUpdate::ItemDelta {
            turn_id: turn_id(),
            item_id: item_id("item-1"),
            delta: ItemDelta::Reasoning { text: "x".into() },
        },
    );
    assert!(matches!(
        accumulator.apply(&update),
        TranscriptApplyResult::Applied(_)
    ));
    assert_eq!(accumulator.apply(&update), TranscriptApplyResult::Ignored);
}

#[test]
fn stream_gap_clears_transient_entries_instead_of_emitting_partial_text() {
    let mut accumulator = TranscriptAccumulator::new(session_id(), thread_id());
    apply(
        &mut accumulator,
        transient(
            1,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-1"),
                delta: ItemDelta::AgentMessage {
                    text: "first".into(),
                },
            },
        ),
    );
    let result = apply(
        &mut accumulator,
        transient(
            3,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-1"),
                delta: ItemDelta::AgentMessage {
                    text: "missing".into(),
                },
            },
        ),
    );
    let TranscriptApplyResult::Applied(update) = result else {
        unreachable!()
    };
    assert_eq!(update.changes, vec![ThreadTranscriptChange::ClearTransient]);

    assert_eq!(
        accumulator.apply(&transient(
            4,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-1"),
                delta: ItemDelta::AgentMessage {
                    text: "still partial".into(),
                },
            },
        )),
        TranscriptApplyResult::Ignored
    );
    assert!(matches!(
        accumulator.apply(&transient_for_stream(
            "stream-2",
            1,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-2"),
                delta: ItemDelta::AgentMessage {
                    text: "complete again".into(),
                },
            },
        )),
        TranscriptApplyResult::Applied(_)
    ));
}

#[test]
fn stdout_and_stderr_stay_independent_until_tool_result_commits() {
    let mut accumulator = TranscriptAccumulator::new(session_id(), thread_id());
    let tool_call_id = ToolCallId::new("call-1").unwrap();
    let stdout = apply(
        &mut accumulator,
        transient(
            1,
            ThreadUpdate::ToolOutputDelta {
                turn_id: turn_id(),
                tool_call_id: tool_call_id.clone(),
                stream: ToolOutputStream::Stdout,
                text: "out".into(),
            },
        ),
    );
    let stderr = apply(
        &mut accumulator,
        transient(
            2,
            ThreadUpdate::ToolOutputDelta {
                turn_id: turn_id(),
                tool_call_id: tool_call_id.clone(),
                stream: ToolOutputStream::Stderr,
                text: "err".into(),
            },
        ),
    );

    let TranscriptApplyResult::Applied(stdout) = stdout else {
        unreachable!()
    };
    let TranscriptApplyResult::Applied(stderr) = stderr else {
        unreachable!()
    };
    assert!(matches!(
        &stdout.changes[0],
        ThreadTranscriptChange::Upsert {
            entry: ThreadTranscriptEntry::ToolOutput {
                stream: ToolOutputStream::Stdout,
                text,
                ..
            }
        } if text == "out"
    ));
    assert!(matches!(
        &stderr.changes[0],
        ThreadTranscriptChange::Upsert {
            entry: ThreadTranscriptEntry::ToolOutput {
                stream: ToolOutputStream::Stderr,
                text,
                ..
            }
        } if text == "err"
    ));

    let committed = apply(
        &mut accumulator,
        durable(
            1,
            ThreadEvent::ItemCompleted {
                thread_id: thread_id(),
                turn_id: turn_id(),
                item: ThreadItem::ToolResult {
                    item_id: item_id("result-1"),
                    turn_id: turn_id(),
                    tool_call_id,
                    text: "done".into(),
                    content: None,
                    is_error: false,
                },
            },
        ),
    );
    let TranscriptApplyResult::Applied(committed) = committed else {
        unreachable!()
    };
    assert!(matches!(
        &committed.changes[0],
        ThreadTranscriptChange::Remove { entry_ids } if entry_ids.len() == 2
    ));
    assert!(matches!(
        &committed.changes[1],
        ThreadTranscriptChange::Upsert {
            entry: ThreadTranscriptEntry::Item {
                transient: false,
                ..
            }
        }
    ));
}

#[test]
fn snapshot_includes_current_transient_entries_for_late_consumers() {
    let mut accumulator = TranscriptAccumulator::new(session_id(), thread_id());
    apply(
        &mut accumulator,
        transient(
            1,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("item-live"),
                delta: ItemDelta::AgentMessage {
                    text: "in progress".into(),
                },
            },
        ),
    );
    let snapshot = accumulator
        .snapshot(&empty_thread())
        .expect("matching Thread scope");

    assert_eq!(snapshot.revision, 1);

    assert!(matches!(
        &snapshot.entries[0],
        ThreadTranscriptEntry::Item {
            item: ThreadItem::AgentMessage { text, .. },
            transient: true,
            ..
        } if text == "in progress"
    ));
}

fn apply(
    accumulator: &mut TranscriptAccumulator,
    update: ThreadUpdateEnvelope,
) -> TranscriptApplyResult {
    accumulator.apply(&update)
}

fn entry_text(result: &TranscriptApplyResult) -> &str {
    let TranscriptApplyResult::Applied(update) = result else {
        panic!("update applied")
    };
    let ThreadTranscriptChange::Upsert {
        entry: ThreadTranscriptEntry::Item { item, .. },
    } = &update.changes[0]
    else {
        panic!("item upsert")
    };
    match item {
        ThreadItem::AgentMessage { text, .. } => text,
        _ => panic!("agent item"),
    }
}

fn revision(result: &TranscriptApplyResult) -> u64 {
    let TranscriptApplyResult::Applied(update) = result else {
        panic!("update applied")
    };
    update.revision
}

fn transient(sequence: u64, update: ThreadUpdate) -> ThreadUpdateEnvelope {
    transient_for_stream("stream-1", sequence, update)
}

fn transient_for_stream(
    stream_id: &str,
    sequence: u64,
    update: ThreadUpdate,
) -> ThreadUpdateEnvelope {
    ThreadUpdateEnvelope {
        session_id: session_id(),
        thread_id: thread_id(),
        durable_sequence: 0,
        stream_cursor: Some(StreamCursor {
            stream_instance_id: StreamInstanceId::new(stream_id).unwrap(),
            sequence,
        }),
        update,
    }
}

fn durable(sequence: u64, event: ThreadEvent) -> ThreadUpdateEnvelope {
    ThreadUpdateEnvelope {
        session_id: session_id(),
        thread_id: thread_id(),
        durable_sequence: sequence,
        stream_cursor: None,
        update: ThreadUpdate::Committed { event },
    }
}

fn agent_item(item: &str, text: &str) -> ThreadItem {
    ThreadItem::AgentMessage {
        item_id: item_id(item),
        turn_id: turn_id(),
        text: text.into(),
    }
}

fn empty_thread() -> Thread {
    Thread {
        session_id: session_id(),
        thread_id: thread_id(),
        parent_thread_id: None,
        forked_from_id: None,
        title: "Thread".into(),
        status: ThreadStatus::Active,
        sequence: 0,
        usage: ModelUsageSummary::default(),
        reference_cost: zeta_protocol::ModelReferenceCostSummary::default(),
        goal: None,
        turns: Vec::new(),
    }
}

fn session_id() -> SessionId {
    SessionId::new("session-1").unwrap()
}

fn thread_id() -> ThreadId {
    ThreadId::new("thread-1").unwrap()
}

fn turn_id() -> TurnId {
    TurnId::new("turn-1").unwrap()
}

fn item_id(value: &str) -> ItemId {
    ItemId::new(value).unwrap()
}
