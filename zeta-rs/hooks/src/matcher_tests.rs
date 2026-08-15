use super::*;
use std::collections::BTreeSet;
use zeta_config::HookAction;
use zeta_config::HookEnablement;
use zeta_config::HookEvent as ConfigHookEvent;
use zeta_config::HookId;
use zeta_config::HookMatcher;
use zeta_core::AfterToolHookRequest;
use zeta_core::BeforeToolHookRequest;
use zeta_core::HookOutcome;
use zeta_core::TurnCompletedHookRequest;
use zeta_protocol::ThreadId;
use zeta_protocol::ToolCallId;
use zeta_protocol::TurnId;

fn hook(event: ConfigHookEvent, tool_names: &[&str]) -> HookConfig {
    HookConfig {
        id: HookId::new("user:hook:matcher").expect("test Hook id"),
        event,
        matcher: HookMatcher {
            tool_names: tool_names
                .iter()
                .map(|name| (*name).into())
                .collect::<BTreeSet<_>>(),
        },
        action: HookAction::Process {
            program: "hook-program".into(),
            args: Vec::new(),
        },
        enablement: HookEnablement::Enabled,
    }
}

fn before_request(tool_name: &str) -> BeforeToolHookRequest {
    BeforeToolHookRequest {
        thread_id: ThreadId::new("thread-test").unwrap(),
        turn_id: TurnId::new("turn-test").unwrap(),
        tool_call_id: ToolCallId::new("tool-test").unwrap(),
        tool_name: tool_name.into(),
    }
}

fn after_request(tool_name: &str, outcome: HookOutcome) -> AfterToolHookRequest {
    AfterToolHookRequest {
        thread_id: ThreadId::new("thread-test").unwrap(),
        turn_id: TurnId::new("turn-test").unwrap(),
        tool_call_id: ToolCallId::new("tool-test").unwrap(),
        tool_name: tool_name.into(),
        outcome,
    }
}

#[test]
fn exact_tool_matcher_applies_only_to_the_declared_tool_event() {
    let hook = hook(ConfigHookEvent::BeforeTool, &["shell-command"]);

    assert!(matches_event(
        &hook,
        &HookInvocation::BeforeTool(&before_request("shell-command"))
    ));
    assert!(!matches_event(
        &hook,
        &HookInvocation::BeforeTool(&before_request("file-system"))
    ));
    assert!(!matches_event(
        &hook,
        &HookInvocation::AfterTool(&after_request("shell-command", HookOutcome::Succeeded))
    ));
}

#[test]
fn empty_matcher_matches_tool_events_but_not_another_event() {
    let hook = hook(ConfigHookEvent::AfterTool, &[]);

    assert!(matches_event(
        &hook,
        &HookInvocation::AfterTool(&after_request("file-system", HookOutcome::Failed))
    ));
    let completed = TurnCompletedHookRequest {
        thread_id: ThreadId::new("thread-test").unwrap(),
        turn_id: TurnId::new("turn-test").unwrap(),
    };
    assert!(!matches_event(
        &hook,
        &HookInvocation::TurnCompleted(&completed)
    ));
}
