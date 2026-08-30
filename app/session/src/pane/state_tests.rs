//! Session Pane state tests.

use super::classification_history_for_thread;
use super::turn_command_was_not_found;
use zeta_input_classifier::InputHistoryEntry;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[test]
fn command_not_found_shell_results_are_excluded_from_classifier_history() {
    let result = |exit_code| ThreadItem::ToolResult {
        item_id: ItemId::new(format!("item-{exit_code}")).unwrap(),
        turn_id: TurnId::new("turn-1").unwrap(),
        tool_call_id: ToolCallId::new("call-1").unwrap(),
        text: serde_json::json!({ "exit_code": exit_code }).to_string(),
        content: None,
        is_error: exit_code != 0,
    };

    assert!(turn_command_was_not_found(&[result(127)]));
    assert!(!turn_command_was_not_found(&[result(1)]));
}

#[test]
fn thread_snapshot_preserves_prompt_and_direct_shell_history_order() {
    let agent_turn_id = TurnId::new("turn-agent").unwrap();
    let shell_turn_id = TurnId::new("turn-shell").unwrap();
    let thread = Thread {
        session_id: SessionId::new("session-1").unwrap(),
        thread_id: ThreadId::new("thread-1").unwrap(),
        parent_thread_id: None,
        forked_from_id: None,
        title: "Thread".to_owned(),
        status: ThreadStatus::Active,
        sequence: 1,
        usage: Default::default(),
        goal: None,
        turns: vec![
            Turn {
                turn_id: agent_turn_id.clone(),
                status: TurnStatus::Completed,
                kind: zeta_protocol::TurnKind::Coding,
                instructions: None,
                model: None,
                tool_profile: None,
                tool_mode: Default::default(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                usage: Default::default(),
                context_usage: None,
                items: vec![ThreadItem::UserMessage {
                    item_id: ItemId::new("item-user").unwrap(),
                    turn_id: agent_turn_id,
                    text: "fix this".to_owned(),
                }],
                plan: None,
                pending_interaction: None,
                error: None,
            },
            Turn {
                turn_id: shell_turn_id.clone(),
                status: TurnStatus::Completed,
                kind: zeta_protocol::TurnKind::Coding,
                instructions: None,
                model: None,
                tool_profile: None,
                tool_mode: Default::default(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                usage: Default::default(),
                context_usage: None,
                items: vec![ThreadItem::ToolCall {
                    item_id: ItemId::new("item-shell").unwrap(),
                    turn_id: shell_turn_id,
                    tool_call_id: ToolCallId::new("call-shell").unwrap(),
                    name: ToolName::new("shell-command").unwrap(),
                    arguments_json: serde_json::json!({
                        "program": "/bin/sh",
                        "arguments": ["-lc", "cargo test"]
                    })
                    .to_string(),
                    binding: None,
                }],
                plan: None,
                pending_interaction: None,
                error: None,
            },
        ],
    };

    assert_eq!(
        classification_history_for_thread(&thread),
        [
            InputHistoryEntry::agent("fix this"),
            InputHistoryEntry::shell("cargo test"),
        ]
    );
}
