use super::{ThreadProjection, ThreadProjectionUpdate};
use zeta_protocol::{
    ItemDelta, ItemId, ModelId, ModelRef, ProviderId, SessionId, StreamCursor, StreamInstanceId,
    Thread, ThreadId, ThreadItem, ThreadStatus, ThreadUpdate, ThreadUpdateEnvelope, ToolCallId,
    ToolOutputStream, Turn, TurnId, TurnStatus,
};

#[test]
fn snapshot_items_are_followed_by_transient_agent_text() {
    let mut projection = ThreadProjection::default();
    projection.replace_snapshot(thread_snapshot(4));

    assert_eq!(
        projection.apply_update(update(
            4,
            1,
            ThreadUpdate::ItemStarted {
                turn_id: turn_id(),
                item: ThreadItem::AgentMessage {
                    item_id: item_id("streaming"),
                    turn_id: turn_id(),
                    text: "hel".to_owned(),
                },
            },
        )),
        ThreadProjectionUpdate::Applied
    );
    assert_eq!(
        projection.apply_update(update(
            4,
            2,
            ThreadUpdate::ItemDelta {
                turn_id: turn_id(),
                item_id: item_id("streaming"),
                delta: ItemDelta::AgentMessage {
                    text: "lo".to_owned(),
                },
            },
        )),
        ThreadProjectionUpdate::Applied
    );

    let text = projection
        .items()
        .map(|item| match item {
            ThreadItem::UserMessage { text, .. } | ThreadItem::AgentMessage { text, .. } => {
                text.as_str()
            }
            _ => "",
        })
        .collect::<Vec<_>>();
    assert_eq!(text, vec!["question", "hello"]);
}

#[test]
fn committed_update_requires_an_authoritative_refresh() {
    let mut projection = ThreadProjection::default();
    projection.replace_snapshot(thread_snapshot(4));

    assert_eq!(
        projection.apply_update(update(
            4,
            1,
            ThreadUpdate::Committed {
                event: zeta_protocol::ThreadEvent::TurnCompleted {
                    thread_id: thread_id(),
                    turn_id: turn_id(),
                },
            },
        )),
        ThreadProjectionUpdate::ResubscribeRequired
    );
}

#[test]
fn stream_gap_discards_transient_state_and_requires_resubscribe() {
    let mut projection = ThreadProjection::default();
    projection.replace_snapshot(thread_snapshot(4));
    let started = ThreadUpdate::ItemStarted {
        turn_id: turn_id(),
        item: ThreadItem::AgentMessage {
            item_id: item_id("streaming"),
            turn_id: turn_id(),
            text: "partial".to_owned(),
        },
    };
    assert_eq!(
        projection.apply_update(update(4, 1, started.clone())),
        ThreadProjectionUpdate::Applied
    );
    assert_eq!(
        projection.apply_update(update(4, 3, started)),
        ThreadProjectionUpdate::ResubscribeRequired
    );
    assert_eq!(projection.items().count(), 1);
}

#[test]
fn update_for_another_thread_is_ignored() {
    let mut projection = ThreadProjection::default();
    projection.replace_snapshot(thread_snapshot(4));
    let mut envelope = update(
        4,
        1,
        ThreadUpdate::ItemStarted {
            turn_id: turn_id(),
            item: ThreadItem::AgentMessage {
                item_id: item_id("other-stream"),
                turn_id: turn_id(),
                text: "ignored".to_owned(),
            },
        },
    );
    envelope.thread_id = ThreadId::new("other").unwrap();

    assert_eq!(
        projection.apply_update(envelope),
        ThreadProjectionUpdate::Ignored
    );
    assert_eq!(projection.items().count(), 1);
}

#[test]
fn snapshot_exposes_the_latest_durable_plan() {
    let mut thread = thread_snapshot(4);
    thread.turns[0].plan = Some(zeta_protocol::PlanUpdate {
        explanation: Some("Check the durable state".to_owned()),
        steps: Vec::new(),
    });
    let mut projection = ThreadProjection::default();

    projection.replace_snapshot(thread);

    assert_eq!(
        projection
            .plan()
            .and_then(|plan| plan.explanation.as_deref()),
        Some("Check the durable state")
    );
}

#[test]
fn typed_tool_output_streams_remain_separate_until_the_durable_result() {
    let mut projection = ThreadProjection::default();
    projection.replace_snapshot(thread_snapshot(4));
    let call_id = ToolCallId::new("call").unwrap();

    assert_eq!(
        projection.apply_update(update(
            4,
            1,
            ThreadUpdate::ToolOutputDelta {
                turn_id: turn_id(),
                tool_call_id: call_id.clone(),
                stream: ToolOutputStream::Stdout,
                text: "out".into(),
            },
        )),
        ThreadProjectionUpdate::Applied
    );
    assert_eq!(
        projection.apply_update(update(
            4,
            2,
            ThreadUpdate::ToolOutputDelta {
                turn_id: turn_id(),
                tool_call_id: call_id.clone(),
                stream: ToolOutputStream::Stderr,
                text: "err".into(),
            },
        )),
        ThreadProjectionUpdate::Applied
    );
    assert_eq!(projection.tool_output(&call_id), Some(("out", "err")));
}

fn thread_snapshot(sequence: u64) -> Thread {
    Thread {
        session_id: session_id(),
        thread_id: thread_id(),
        title: "Agent thread".to_owned(),
        status: ThreadStatus::Active,
        sequence,
        usage: Default::default(),
        turns: vec![Turn {
            turn_id: turn_id(),
            status: TurnStatus::Running,
            model: Some(ModelRef {
                provider: ProviderId::new("test").unwrap(),
                model: ModelId::new("test-model").unwrap(),
            }),
            resource_budget: None,
            tool_profile: None,
            usage: Default::default(),
            items: vec![ThreadItem::UserMessage {
                item_id: item_id("question"),
                turn_id: turn_id(),
                text: "question".to_owned(),
            }],
            plan: None,
            pending_interaction: None,
            error: None,
        }],
    }
}

fn update(
    durable_sequence: u64,
    stream_sequence: u64,
    update: ThreadUpdate,
) -> ThreadUpdateEnvelope {
    ThreadUpdateEnvelope {
        session_id: session_id(),
        thread_id: thread_id(),
        durable_sequence,
        stream_cursor: Some(StreamCursor {
            stream_instance_id: StreamInstanceId::new("stream").unwrap(),
            sequence: stream_sequence,
        }),
        update,
    }
}

fn session_id() -> SessionId {
    SessionId::new("session").unwrap()
}

fn thread_id() -> ThreadId {
    ThreadId::new("thread").unwrap()
}

fn turn_id() -> TurnId {
    TurnId::new("turn").unwrap()
}

fn item_id(value: &str) -> ItemId {
    ItemId::new(value).unwrap()
}
