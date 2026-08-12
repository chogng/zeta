use super::compose_dynamic_tools;
use serde_json::json;
use zeta_async_utils::CancellationSource;
use zeta_policy::ExecutionDecision;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::DynamicToolOutput;
use zeta_protocol::DynamicToolResponse;
use zeta_protocol::DynamicToolSpec;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolName;

fn specification(description: &str) -> DynamicToolSpec {
    DynamicToolSpec {
        name: ToolName::new("client_lookup").unwrap(),
        description: description.into(),
        input_schema: json!({
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": ["query"]
        }),
    }
}

fn call() -> ToolCall {
    ToolCall {
        id: ToolCallId::new("call-1").unwrap(),
        name: ToolName::new("client_lookup").unwrap(),
        arguments: json!({"query": "zeta"}),
    }
}

#[test]
fn dynamic_tool_requires_approval_then_round_trips_the_frozen_call() {
    let composition = compose_dynamic_tools(vec![specification("Look up client state.")])
        .unwrap()
        .unwrap();
    let call = call();
    let reviewed = composition.tools.prepare(&call).unwrap();
    assert!(matches!(
        composition
            .policy
            .decide(&reviewed, &CancellationSource::new().token()),
        Ok(ExecutionDecision::AskUser(_))
    ));

    let request = composition
        .tools
        .execution_interaction(&call)
        .unwrap()
        .unwrap();
    let AgentRequest::DynamicTool { call: dynamic_call } = &request else {
        panic!("dynamic tool must produce a dynamic interaction")
    };
    assert_eq!(dynamic_call.call_id, call.id);
    assert_eq!(dynamic_call.name, call.name);
    assert_eq!(dynamic_call.arguments, call.arguments);
    assert_eq!(dynamic_call.definition_digest.len(), 64);

    let response = AgentResponse::DynamicTool {
        response: DynamicToolResponse {
            call_id: call.id.clone(),
            content: vec![DynamicToolOutput::Text {
                text: "found".into(),
            }],
            success: true,
        },
    };
    let output = composition
        .tools
        .resolve_execution_interaction(&call, &request, &response)
        .unwrap();
    assert!(matches!(
        output,
        Some(ToolExecutionOutput::SuccessContent(content))
            if content == vec![zeta_protocol::ContentPart::Text("found".into())]
    ));
}

#[test]
fn changed_same_name_definition_cannot_claim_an_old_interaction() {
    let old = compose_dynamic_tools(vec![specification("Old definition.")])
        .unwrap()
        .unwrap();
    let current = compose_dynamic_tools(vec![specification("Changed definition.")])
        .unwrap()
        .unwrap();
    let call = call();
    let request = old.tools.execution_interaction(&call).unwrap().unwrap();
    let response = AgentResponse::DynamicTool {
        response: DynamicToolResponse {
            call_id: call.id.clone(),
            content: vec![DynamicToolOutput::Text {
                text: "stale".into(),
            }],
            success: true,
        },
    };
    let error = current
        .tools
        .resolve_execution_interaction(&call, &request, &response)
        .unwrap_err();
    assert!(error.to_string().contains("frozen Tool Call binding"));
}
