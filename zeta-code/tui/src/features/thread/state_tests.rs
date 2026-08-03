use super::ThreadFeatureState;
use crate::components::transcript::CommandStatus;
use crate::components::transcript::MessageRole;
use crate::features::thread::ThreadPresentationEvent;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
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
            (MessageRole::User, "[Image]"),
            (MessageRole::Agent, "canonical response"),
        ]
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
                    text: "not currently rendered".into(),
                },
                ThreadItem::UserImage {
                    item_id: ItemId::new("item_3").unwrap(),
                    turn_id: turn_id.clone(),
                    url: "data:image/png;base64,cG5n".into(),
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
