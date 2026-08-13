use super::ThreadFeatureState;
use crate::components::transcript::CommandStatus;
use crate::components::transcript::MessageRole;
use crate::features::thread::ThreadPresentationEvent;
use zeta_protocol::ContentDigest;
use zeta_protocol::ImageAttachmentRef;
use zeta_protocol::ImageMediaType;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[test]
fn canonical_snapshot_replaces_optimistic_projection_and_preserves_identity() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::UserSubmitted("optimistic".into()));
    state.update(ThreadPresentationEvent::SnapshotReceived(thread_snapshot()));

    assert_eq!(state.snapshot().unwrap().thread_id.as_str(), "thread_1");
    assert_eq!(state.snapshot().unwrap().sequence, 7);
    assert_eq!(
        state
            .messages()
            .iter()
            .map(|message| (message.role, message.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (MessageRole::User, "canonical prompt"),
            (MessageRole::Reasoning, "inspect the code"),
            (MessageRole::User, "[Image]"),
            (MessageRole::User, "[Image]"),
            (MessageRole::Plan, "1. inspect\n2. change"),
            (MessageRole::Tool, "Tool · read_file"),
            (MessageRole::Tool, "Tool result · read_file"),
            (MessageRole::Agent, "canonical response"),
        ]
    );
    assert_eq!(
        state.messages()[5].detail.as_deref(),
        Some("{\n  \"path\": \"src/lib.rs\"\n}")
    );
    assert_eq!(state.messages()[6].detail.as_deref(), Some("file contents"));
}

#[test]
fn older_history_page_is_merged_before_the_loaded_snapshot() {
    let mut state = ThreadFeatureState::default();
    let current = thread_snapshot();
    state.update(ThreadPresentationEvent::SnapshotReceived(current.clone()));
    let older_turn_id = TurnId::new("turn_0").unwrap();
    state.update(ThreadPresentationEvent::HistoryPageReceived(Thread {
        sequence: 99,
        turns: vec![Turn {
            turn_id: older_turn_id.clone(),
            status: TurnStatus::Completed,
            model: None,
            items: vec![ThreadItem::UserMessage {
                item_id: ItemId::new("older_item").unwrap(),
                turn_id: older_turn_id,
                text: "older prompt".into(),
            }],
            pending_interaction: None,
            error: None,
        }],
        ..current
    }));

    let snapshot = state.snapshot().unwrap();
    assert_eq!(snapshot.sequence, 7);
    assert_eq!(snapshot.turns.len(), 2);
    assert_eq!(snapshot.turns[0].turn_id.as_str(), "turn_0");
    assert_eq!(snapshot.turns[1].turn_id.as_str(), "turn_1");
    assert_eq!(state.messages()[0].text, "older prompt");
}

#[test]
fn older_history_page_preserves_the_active_transient_projection() {
    let mut state = ThreadFeatureState::default();
    let current = thread_snapshot();
    state.update(ThreadPresentationEvent::SnapshotReceived(current.clone()));
    let turn_id = TurnId::new("turn_stream").unwrap();
    let item_id = ItemId::new("item_stream").unwrap();
    state.update(ThreadPresentationEvent::TransientUpdateReceived(Box::new(
        transient(ThreadUpdate::ItemDelta {
            turn_id: turn_id.clone(),
            item_id: item_id.clone(),
            delta: zeta_protocol::ItemDelta::AgentMessage {
                text: "stream".into(),
            },
        }),
    )));

    let older_turn_id = TurnId::new("turn_0").unwrap();
    state.update(ThreadPresentationEvent::HistoryPageReceived(Thread {
        turns: vec![Turn {
            turn_id: older_turn_id.clone(),
            status: TurnStatus::Completed,
            model: None,
            items: vec![ThreadItem::UserMessage {
                item_id: ItemId::new("older_item").unwrap(),
                turn_id: older_turn_id,
                text: "older prompt".into(),
            }],
            pending_interaction: None,
            error: None,
        }],
        ..current
    }));
    state.update(ThreadPresentationEvent::TransientUpdateReceived(Box::new(
        transient(ThreadUpdate::ItemDelta {
            turn_id,
            item_id,
            delta: zeta_protocol::ItemDelta::AgentMessage { text: "ing".into() },
        }),
    )));

    assert_eq!(state.messages()[0].text, "older prompt");
    assert_eq!(state.messages().last().unwrap().text, "streaming");
}

#[test]
fn transient_deltas_update_identity_stable_transcript_rows() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::SnapshotReceived(Thread {
        turns: Vec::new(),
        ..thread_snapshot()
    }));
    let turn_id = TurnId::new("turn_stream").unwrap();
    let item_id = ItemId::new("item_stream").unwrap();
    state.update(ThreadPresentationEvent::TransientUpdateReceived(Box::new(
        transient(ThreadUpdate::ItemStarted {
            turn_id: turn_id.clone(),
            item: ThreadItem::AgentMessage {
                item_id: item_id.clone(),
                turn_id: turn_id.clone(),
                text: String::new(),
            },
        }),
    )));
    for text in ["hel", "lo"] {
        state.update(ThreadPresentationEvent::TransientUpdateReceived(Box::new(
            transient(ThreadUpdate::ItemDelta {
                turn_id: turn_id.clone(),
                item_id: item_id.clone(),
                delta: zeta_protocol::ItemDelta::AgentMessage { text: text.into() },
            }),
        )));
    }

    assert_eq!(state.messages().len(), 1);
    assert_eq!(state.messages()[0].role, MessageRole::Agent);
    assert_eq!(state.messages()[0].text, "hello");
}

#[test]
fn transient_projection_bounds_each_message_without_splitting_utf8() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::SnapshotReceived(Thread {
        turns: Vec::new(),
        ..thread_snapshot()
    }));
    let turn_id = TurnId::new("turn_stream").unwrap();
    let item_id = ItemId::new("item_stream").unwrap();
    state.update(ThreadPresentationEvent::TransientUpdateReceived(Box::new(
        transient(ThreadUpdate::ItemDelta {
            turn_id,
            item_id,
            delta: zeta_protocol::ItemDelta::AgentMessage {
                text: "界".repeat(100_000),
            },
        }),
    )));

    let text = &state.messages()[0].text;
    assert!(text.len() <= 256 * 1024);
    assert!(text.ends_with("… transient output truncated …"));
    assert!(std::str::from_utf8(text.as_bytes()).is_ok());
}

#[test]
fn transient_projection_bounds_identity_cardinality() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::SnapshotReceived(Thread {
        turns: Vec::new(),
        ..thread_snapshot()
    }));
    let turn_id = TurnId::new("turn_stream").unwrap();
    for index in 0..1_100 {
        state.update(ThreadPresentationEvent::TransientUpdateReceived(Box::new(
            transient(ThreadUpdate::ItemDelta {
                turn_id: turn_id.clone(),
                item_id: ItemId::new(format!("item_{index}")).unwrap(),
                delta: zeta_protocol::ItemDelta::AgentMessage { text: "x".into() },
            }),
        )));
    }

    assert_eq!(state.messages().len(), 1_024);
    assert_eq!(
        state.messages()[0].source_id.as_deref(),
        Some("item:item_76")
    );
}

#[test]
fn transient_stream_reset_removes_only_transient_rows() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::SnapshotReceived(thread_snapshot()));
    state.update(ThreadPresentationEvent::NoticeReceived(
        "local notice".into(),
    ));
    state.update(ThreadPresentationEvent::TransientUpdateReceived(Box::new(
        transient(ThreadUpdate::ItemDelta {
            turn_id: TurnId::new("turn_stream").unwrap(),
            item_id: ItemId::new("item_stream").unwrap(),
            delta: zeta_protocol::ItemDelta::AgentMessage {
                text: "temporary".into(),
            },
        }),
    )));

    state.update(ThreadPresentationEvent::TransientStreamReset);

    assert!(
        state
            .messages()
            .iter()
            .all(|message| message.text != "temporary")
    );
    assert!(
        state
            .messages()
            .iter()
            .any(|message| message.text == "canonical response")
    );
    assert!(
        state
            .messages()
            .iter()
            .any(|message| message.text == "local notice")
    );
}

#[test]
fn local_presentation_events_share_the_thread_owner() {
    let mut state = ThreadFeatureState::default();

    state.update(ThreadPresentationEvent::NoticeReceived("notice".into()));
    state.update(ThreadPresentationEvent::FailureReported("failure".into()));
    state.update(ThreadPresentationEvent::Interrupted);

    assert_eq!(
        state
            .messages()
            .iter()
            .map(|message| (message.role, message.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (MessageRole::Notice, "notice"),
            (MessageRole::Error, "failure"),
            (MessageRole::Notice, "turn interrupted"),
        ]
    );
}

#[test]
fn command_completion_groups_the_command_with_its_result() {
    let mut state = ThreadFeatureState::default();

    state.update(ThreadPresentationEvent::CommandStarted(
        "/theme zeta-code-light".into(),
    ));
    let running = state.messages().first().unwrap();
    assert_eq!(running.command_status, Some(CommandStatus::Running));
    assert_eq!(running.detail, None);

    state.update(ThreadPresentationEvent::CommandCompleted {
        command: "/theme zeta-code-light".into(),
        result: "Theme set to Zeta Code Light".into(),
    });

    let message = state.messages().first().unwrap();
    assert_eq!(state.messages().len(), 1);
    assert_eq!(message.role, MessageRole::Command);
    assert_eq!(message.text, "/theme zeta-code-light");
    assert_eq!(
        message.detail.as_deref(),
        Some("Theme set to Zeta Code Light")
    );
    assert_eq!(message.command_status, Some(CommandStatus::Succeeded));
}

#[test]
fn clearing_discards_snapshot_and_projection() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::SnapshotReceived(thread_snapshot()));

    state.update(ThreadPresentationEvent::Cleared);

    assert_eq!(state.snapshot(), None);
    assert!(state.messages().is_empty());
}

fn thread_snapshot() -> Thread {
    let turn_id = TurnId::new("turn_1").unwrap();
    Thread {
        session_id: SessionId::new("session_1").unwrap(),
        thread_id: ThreadId::new("thread_1").unwrap(),
        title: "Thread".into(),
        status: ThreadStatus::Active,
        sequence: 7,
        turns: vec![Turn {
            turn_id: turn_id.clone(),
            status: TurnStatus::Completed,
            model: None,
            items: vec![
                ThreadItem::UserMessage {
                    item_id: ItemId::new("item_1").unwrap(),
                    turn_id: turn_id.clone(),
                    text: "canonical prompt".into(),
                },
                ThreadItem::Reasoning {
                    item_id: ItemId::new("item_2").unwrap(),
                    turn_id: turn_id.clone(),
                    text: "inspect the code".into(),
                },
                ThreadItem::UserImage {
                    item_id: ItemId::new("item_3").unwrap(),
                    turn_id: turn_id.clone(),
                    url: "data:image/png;base64,cG5n".into(),
                },
                ThreadItem::UserImageAttachment {
                    item_id: ItemId::new("item_attachment").unwrap(),
                    turn_id: turn_id.clone(),
                    attachment: ImageAttachmentRef {
                        content_digest: ContentDigest::sha256(b"png"),
                        media_type: ImageMediaType::Png,
                        encoded_bytes: 3,
                        width: 1,
                        height: 1,
                    },
                },
                ThreadItem::Plan {
                    item_id: ItemId::new("item_plan").unwrap(),
                    turn_id: turn_id.clone(),
                    text: "1. inspect\n2. change".into(),
                },
                ThreadItem::ToolCall {
                    item_id: ItemId::new("item_tool_call").unwrap(),
                    turn_id: turn_id.clone(),
                    tool_call_id: ToolCallId::new("call_1").unwrap(),
                    name: ToolName::new("read_file").unwrap(),
                    arguments_json: r#"{"path":"src/lib.rs"}"#.into(),
                    binding: None,
                },
                ThreadItem::ToolResult {
                    item_id: ItemId::new("item_tool_result").unwrap(),
                    turn_id: turn_id.clone(),
                    tool_call_id: ToolCallId::new("call_1").unwrap(),
                    text: "file contents".into(),
                    content: None,
                    is_error: false,
                },
                ThreadItem::AgentMessage {
                    item_id: ItemId::new("item_4").unwrap(),
                    turn_id,
                    text: "canonical response".into(),
                },
            ],
            pending_interaction: None,
            error: None,
        }],
    }
}

fn transient(update: ThreadUpdate) -> ThreadUpdateEnvelope {
    ThreadUpdateEnvelope {
        session_id: SessionId::new("session_1").unwrap(),
        thread_id: ThreadId::new("thread_1").unwrap(),
        durable_sequence: 7,
        stream_cursor: None,
        update,
    }
}
