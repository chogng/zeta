use super::*;
use zeta_config::HookAction;
use zeta_config::HookEnablement;
use zeta_config::HookEvent;
use zeta_config::HookId;
use zeta_config::HookMatcher;
use zeta_core::BeforeToolHookRequest;
use zeta_protocol::ThreadId;
use zeta_protocol::ToolCallId;
use zeta_protocol::TurnId;

#[test]
fn input_uses_the_zeta_protocol_and_canonical_safe_point_identity() {
    let hook = HookConfig {
        id: HookId::new("user:hook:audit").unwrap(),
        event: HookEvent::BeforeTool,
        matcher: HookMatcher::default(),
        action: HookAction::Process {
            program: "audit-hook".into(),
            args: Vec::new(),
        },
        enablement: HookEnablement::Enabled,
    };
    let request = BeforeToolHookRequest {
        thread_id: ThreadId::new("thread-7").unwrap(),
        turn_id: TurnId::new("turn-3").unwrap(),
        tool_call_id: ToolCallId::new("tool-9").unwrap(),
        tool_name: "shell-command".into(),
    };

    let bytes = encode_input(
        &hook,
        &HookInvocation::BeforeTool(&request),
        Path::new("/workspace"),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(value["protocolVersion"], 1);
    assert_eq!(value["hookId"], "user:hook:audit");
    assert_eq!(value["workspace"], "/workspace");
    assert_eq!(value["event"]["name"], "beforeTool");
    assert_eq!(value["event"]["threadId"], "thread-7");
    assert_eq!(value["event"]["turnId"], "turn-3");
    assert_eq!(value["event"]["toolCallId"], "tool-9");
    assert_eq!(value["event"]["toolName"], "shell-command");
}
