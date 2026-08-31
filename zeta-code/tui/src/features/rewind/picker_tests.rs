use super::RewindSelectionAction;
use super::rewind_choices;
use crate::components::list_selection::ListSelectionState;
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
fn rewind_region_lists_user_message_checkpoints_and_selects_the_latest() {
    let thread = thread(&["first checkpoint", "second checkpoint"]);

    let view = rewind_choices(&thread);
    let state = ListSelectionState::new(view.model.into_body());

    assert_eq!(state.title(), "Rewind");
    assert!(state.search().is_some());
    assert_eq!(
        state
            .visible_items()
            .iter()
            .map(|item| item.label())
            .collect::<Vec<_>>(),
        vec!["1. first checkpoint", "2. second checkpoint"]
    );
    assert_eq!(state.selected_visible_index(), Some(1));
    assert_eq!(
        view.actions.values().last(),
        Some(&RewindSelectionAction::Rewind {
            before_turn_id: TurnId::new("turn-2").unwrap(),
            checkpoint_label: "second checkpoint".into(),
        })
    );
}

fn thread(messages: &[&str]) -> Thread {
    Thread {
        session_id: SessionId::new("session").unwrap(),
        thread_id: ThreadId::new("thread").unwrap(),
        parent_thread_id: None,
        forked_from_id: None,
        title: "thread".into(),
        status: ThreadStatus::Active,
        sequence: 1,
        usage: zeta_protocol::ModelUsageSummary::default(),
        goal: None,
        turns: messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                let ordinal = index + 1;
                let turn_id = TurnId::new(format!("turn-{ordinal}")).unwrap();
                Turn {
                    turn_id: turn_id.clone(),
                    status: TurnStatus::Completed,
                    kind: Default::default(),
                    instructions: None,
                    model: None,
                    tool_profile: None,
                    tool_mode: zeta_protocol::ToolMode::Direct,
                    approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                    usage: zeta_protocol::ModelUsageSummary::default(),
                    context_usage: None,
                    items: vec![ThreadItem::UserMessage {
                        item_id: ItemId::new(format!("item-{ordinal}")).unwrap(),
                        turn_id,
                        text: (*message).into(),
                    }],
                    plan: None,
                    pending_interaction: None,
                    error: None,
                }
            })
            .collect(),
    }
}
