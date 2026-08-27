use super::ThreadFeatureState;
use crate::components::transcript::CommandStatus;
use crate::components::transcript::MessageRole;
use crate::features::thread::ThreadPresentationEvent;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptChange;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptEntry;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope;
use zeta_protocol::ItemId;
use zeta_protocol::PlanStep;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::PlanUpdate;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[test]
fn transcript_snapshot_replaces_local_rows_and_preserves_rendering() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::UserSubmitted("optimistic".into()));
    state.update(ThreadPresentationEvent::TranscriptSnapshotReceived(
        ThreadTranscriptSnapshot::from_thread(&thread_snapshot()),
    ));
    assert_eq!(
        state
            .messages()
            .iter()
            .map(|message| (message.role, message.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (MessageRole::User, "canonical prompt"),
            (MessageRole::Reasoning, "inspect the code"),
            (MessageRole::Agent, "canonical response"),
        ]
    );
}

#[test]
fn history_snapshot_is_prepended_without_duplicate_entries() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::TranscriptSnapshotReceived(
        ThreadTranscriptSnapshot::from_thread(&thread_snapshot()),
    ));
    let older = thread_with_item("turn_0", "older_item", "older prompt");
    state.update(ThreadPresentationEvent::TranscriptHistoryPageReceived(
        ThreadTranscriptSnapshot::from_thread(&older),
    ));
    assert_eq!(state.messages()[0].text, "older prompt");
    assert_eq!(state.messages().len(), 4);
}

#[test]
fn complete_upsert_replaces_one_stable_transcript_row() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::TranscriptSnapshotReceived(
        empty_snapshot(),
    ));
    state.update(ThreadPresentationEvent::TranscriptUpdateReceived(Box::new(
        update(vec![upsert_agent("stream item", true)]),
    )));
    state.update(ThreadPresentationEvent::TranscriptUpdateReceived(Box::new(
        update(vec![upsert_agent("complete text", true)]),
    )));
    assert_eq!(state.messages().len(), 1);
    assert_eq!(state.messages()[0].text, "complete text");
    assert_eq!(
        state.messages()[0].source_id.as_deref(),
        Some("item:item_stream")
    );
}

#[test]
fn clear_transient_preserves_committed_and_local_rows() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::TranscriptSnapshotReceived(
        ThreadTranscriptSnapshot::from_thread(&thread_snapshot()),
    ));
    state.update(ThreadPresentationEvent::NoticeReceived(
        "local notice".into(),
    ));
    state.update(ThreadPresentationEvent::TranscriptUpdateReceived(Box::new(
        update(vec![upsert_agent("temporary", true)]),
    )));
    state.update(ThreadPresentationEvent::TranscriptUpdateReceived(Box::new(
        update(vec![ThreadTranscriptChange::ClearTransient]),
    )));
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
fn structured_turn_plan_is_rendered_by_the_tui() {
    let mut thread = thread_snapshot();
    thread.turns[0].plan = Some(PlanUpdate {
        explanation: Some("Implementation plan".into()),
        steps: vec![
            PlanStep {
                step: "inspect".into(),
                status: PlanStepStatus::Completed,
            },
            PlanStep {
                step: "change".into(),
                status: PlanStepStatus::InProgress,
            },
        ],
    });
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::TranscriptSnapshotReceived(
        ThreadTranscriptSnapshot::from_thread(&thread),
    ));
    let plan = state.messages().last().unwrap();
    assert_eq!(plan.role, MessageRole::Plan);
    assert_eq!(plan.text, "Implementation plan\n[x] inspect\n[>] change");
    assert_eq!(plan.source_id.as_deref(), Some("turn-plan:turn_1"));
}

#[test]
fn command_completion_groups_the_command_with_its_result() {
    let mut state = ThreadFeatureState::default();
    state.update(ThreadPresentationEvent::CommandStarted(
        "/theme light".into(),
    ));
    state.update(ThreadPresentationEvent::CommandCompleted {
        command: "/theme light".into(),
        result: "Theme set".into(),
    });
    let message = state.messages().first().unwrap();
    assert_eq!(message.command_status, Some(CommandStatus::Succeeded));
    assert_eq!(message.detail.as_deref(), Some("Theme set"));
}

fn update(changes: Vec<ThreadTranscriptChange>) -> ThreadTranscriptUpdateEnvelope {
    ThreadTranscriptUpdateEnvelope {
        session_id: session_id(),
        thread_id: thread_id(),
        durable_sequence: 7,
        stream_cursor: None,
        changes,
    }
}

fn upsert_agent(text: &str, transient: bool) -> ThreadTranscriptChange {
    let turn_id = TurnId::new("turn_stream").unwrap();
    let item_id = ItemId::new("item_stream").unwrap();
    ThreadTranscriptChange::Upsert {
        entry: ThreadTranscriptEntry::Item {
            entry_id: "item:item_stream".into(),
            turn_id: turn_id.clone(),
            item: ThreadItem::AgentMessage {
                item_id,
                turn_id,
                text: text.into(),
            },
            transient,
        },
    }
}

fn empty_snapshot() -> ThreadTranscriptSnapshot {
    ThreadTranscriptSnapshot {
        session_id: session_id(),
        thread_id: thread_id(),
        durable_sequence: 7,
        entries: Vec::new(),
    }
}

fn thread_snapshot() -> Thread {
    let turn_id = TurnId::new("turn_1").unwrap();
    Thread {
        session_id: session_id(),
        thread_id: thread_id(),
        title: "Thread".into(),
        status: ThreadStatus::Active,
        sequence: 7,
        usage: zeta_protocol::ModelUsageSummary::default(),
        goal: None,
        turns: vec![Turn {
            turn_id: turn_id.clone(),
            status: TurnStatus::Completed,
            model: None,
            tool_profile: None,
            tool_mode: zeta_protocol::ToolMode::Direct,
            usage: zeta_protocol::ModelUsageSummary::default(),
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
                ThreadItem::AgentMessage {
                    item_id: ItemId::new("item_3").unwrap(),
                    turn_id,
                    text: "canonical response".into(),
                },
            ],
            plan: None,
            pending_interaction: None,
            error: None,
        }],
    }
}

fn thread_with_item(turn: &str, item: &str, text: &str) -> Thread {
    let turn_id = TurnId::new(turn).unwrap();
    Thread {
        turns: vec![Turn {
            turn_id: turn_id.clone(),
            status: TurnStatus::Completed,
            model: None,
            tool_profile: None,
            tool_mode: zeta_protocol::ToolMode::Direct,
            usage: zeta_protocol::ModelUsageSummary::default(),
            items: vec![ThreadItem::UserMessage {
                item_id: ItemId::new(item).unwrap(),
                turn_id,
                text: text.into(),
            }],
            plan: None,
            pending_interaction: None,
            error: None,
        }],
        ..thread_snapshot()
    }
}

fn session_id() -> SessionId {
    SessionId::new("session_1").unwrap()
}
fn thread_id() -> ThreadId {
    ThreadId::new("thread_1").unwrap()
}
