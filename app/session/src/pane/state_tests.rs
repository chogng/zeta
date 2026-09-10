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
use zeta_thread_transcript::ThreadTranscriptSnapshot;

#[test]
fn switching_threads_cancels_submission_even_with_identical_history() {
    use crate::ComposerClassificationUpdate;
    use crate::ComposerSubmission;
    use std::time::Duration;
    use std::time::Instant;

    let mut pane = super::SessionPaneState::default();
    let mut thread = Thread {
        session_id: SessionId::new("classification-session").unwrap(),
        thread_id: ThreadId::new("first-thread").unwrap(),
        parent_thread_id: None,
        forked_from_id: None,
        title: "Classification".to_owned(),
        status: ThreadStatus::Active,
        sequence: 1,
        usage: Default::default(),
        reference_cost: Default::default(),
        goal: None,
        turns: Vec::new(),
    };
    pane.replace_thread(
        thread.clone(),
        ThreadTranscriptSnapshot::from_thread(&thread),
        0,
    );
    pane.set_composer_text("git status 是做什么的");
    assert!(pane.request_composer_submission().is_none());
    let first = pane
        .take_composer_classification_task(Instant::now())
        .unwrap();
    // A repeated snapshot of the same Thread must preserve the pending Enter request.
    pane.replace_thread(
        thread.clone(),
        ThreadTranscriptSnapshot::from_thread(&thread),
        0,
    );
    let result = std::thread::spawn(move || first.run()).join().unwrap();
    assert_eq!(
        pane.finish_composer_classification(result),
        ComposerClassificationUpdate::Submit
    );
    assert!(matches!(
        pane.request_composer_submission(),
        Some(ComposerSubmission::AgentMessage(_))
    ));

    pane.set_composer_text("git status 是做什么的？");
    assert!(pane.request_composer_submission().is_none());
    let old = pane
        .take_composer_classification_task(Instant::now())
        .unwrap();
    thread.thread_id = ThreadId::new("second-thread").unwrap();
    pane.replace_thread(
        thread.clone(),
        ThreadTranscriptSnapshot::from_thread(&thread),
        0,
    );
    assert_eq!(
        pane.finish_composer_classification(old.run()),
        ComposerClassificationUpdate::Stale
    );
    let latest = pane
        .take_composer_classification_task(Instant::now() + Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        pane.finish_composer_classification(latest.run()),
        ComposerClassificationUpdate::Updated
    );
}

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
        reference_cost: Default::default(),
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
