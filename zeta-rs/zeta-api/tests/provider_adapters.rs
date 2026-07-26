use serde_json::{Value, json};
use std::sync::Mutex;
use zeta_api::{
    ApiEndpoint, ApiProtocol, ModelRequest, ReasoningConfig, ReasoningEffort, StopReason,
    ToolDefinition, ToolName,
};
use zeta_client::{ClientError, ClientRequest, ClientResponse, OperationClient, ResolvedApiTarget};
use zeta_http_client::HttpHeader;

struct CapturingTransport {
    request: Mutex<Option<(String, Vec<HttpHeader>, Value)>>,
    response: Value,
}

impl CapturingTransport {
    fn new(response: Value) -> Self {
        Self {
            request: Mutex::new(None),
            response,
        }
    }
}

impl OperationClient for CapturingTransport {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError> {
        let body = serde_json::from_slice(request.body())
            .map_err(|_| ClientError::InvalidRequest("API codec did not produce JSON".into()))?;
        *self.request.lock().unwrap() =
            Some((request.url().into(), request.headers().to_vec(), body));
        let response = serde_json::to_vec(&self.response)
            .map_err(|_| ClientError::InvalidResponse("test response did not encode".into()))?;
        Ok(ClientResponse::new(200, Vec::new(), response))
    }
}

fn target() -> ResolvedApiTarget {
    ResolvedApiTarget::new(
        "https://example.test/v1",
        vec![HttpHeader::new("Authorization", "Bearer secret")],
    )
}

fn tool_request() -> ModelRequest {
    let mut request = ModelRequest::text("weather");
    request.instructions = Some("Use tools when needed.".into());
    request.tools.push(ToolDefinition {
        name: ToolName::new("weather").expect("test tool name is valid"),
        description: "Get weather".into(),
        parameters: json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"],
            "additionalProperties": false,
        }),
        strict: true,
    });
    request.reasoning = Some(ReasoningConfig {
        effort: ReasoningEffort::Medium,
        summary: true,
    });
    request
}

#[test]
fn openai_responses_converts_tools_reasoning_and_tool_calls() {
    let transport = CapturingTransport::new(json!({
        "id": "resp_1",
        "status": "completed",
        "output": [{
            "type": "function_call",
            "call_id": "call_1",
            "name": "weather",
            "arguments": "{\"city\":\"Paris\"}"
        }],
        "usage": {
            "input_tokens": 20,
            "output_tokens": 5,
            "input_tokens_details": {"cached_tokens": 3},
            "output_tokens_details": {"reasoning_tokens": 2}
        }
    }));
    let response = ApiEndpoint::OpenAiResponses
        .complete_with_client(&target(), "gpt-test", &tool_request(), &transport)
        .unwrap();

    let (endpoint, headers, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/responses");
    assert_eq!(headers[0].value(), "Bearer secret");
    assert_eq!(request["input"][0]["content"][0]["type"], "input_text");
    assert_eq!(request["tools"][0]["name"], "weather");
    assert_eq!(request["reasoning"]["effort"], "medium");
    assert_eq!(response.stop_reason, StopReason::ToolUse);
    assert_eq!(
        response.tool_calls().next().unwrap().arguments,
        json!({"city": "Paris"})
    );
    assert_eq!(response.usage.unwrap().reasoning_tokens, 2);
}

#[test]
fn chat_completions_endpoint_converts_tools_and_text() {
    let transport = CapturingTransport::new(json!({
        "id": "chatcmpl_1",
        "choices": [{
            "message": {"content": "sunny"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 4, "completion_tokens": 2}
    }));
    let mut request = tool_request();
    request.reasoning = None;
    let response = ApiEndpoint::OpenAiChatCompletions
        .complete_with_client(&target(), "qwen-test", &request, &transport)
        .unwrap();

    let (endpoint, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/chat/completions");
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["tools"][0]["function"]["name"], "weather");
    assert_eq!(response.text(), "sunny");
}

#[test]
fn anthropic_messages_converts_tools_and_tool_use() {
    let transport = CapturingTransport::new(json!({
        "id": "msg_1",
        "content": [{
            "type": "tool_use",
            "id": "toolu_1",
            "name": "weather",
            "input": {"city": "Paris"}
        }],
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 10, "output_tokens": 4}
    }));
    let mut request = tool_request();
    request.reasoning = None;
    let response = ApiEndpoint::AnthropicMessages
        .complete_with_client(
            &ResolvedApiTarget::new(
                "https://api.anthropic.com",
                vec![HttpHeader::new("x-api-key", "secret")],
            ),
            "claude-test",
            &request,
            &transport,
        )
        .unwrap();

    let (endpoint, headers, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://api.anthropic.com/v1/messages");
    assert!(
        headers
            .iter()
            .any(|header| header.name() == "anthropic-version")
    );
    assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
    assert_eq!(response.stop_reason, StopReason::ToolUse);
    assert_eq!(
        response.tool_calls().next().unwrap().name.as_str(),
        "weather"
    );
}

#[test]
fn endpoint_families_report_their_underlying_protocol() {
    assert_eq!(
        ApiEndpoint::OpenAiResponses.protocol(),
        ApiProtocol::OpenAiResponses
    );
    assert_eq!(
        ApiEndpoint::AnthropicMessages.protocol(),
        ApiProtocol::AnthropicMessages
    );
    assert_eq!(
        ApiEndpoint::OpenAiChatCompletions.protocol(),
        ApiProtocol::OpenAiCompletions
    );
}

#[test]
fn chat_completions_endpoint_dispatches_through_the_shared_codec() {
    let transport = CapturingTransport::new(json!({
        "id": "chatcmpl_1",
        "choices": [{
            "message": {"content": "compatible"},
            "finish_reason": "stop"
        }]
    }));
    let response = ApiEndpoint::OpenAiChatCompletions
        .complete_with_client(
            &target(),
            "compatible-model",
            &ModelRequest::text("hello"),
            &transport,
        )
        .unwrap();
    let (endpoint, _, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/chat/completions");
    assert_eq!(response.text(), "compatible");
}

#[test]
fn header_debug_output_redacts_credentials() {
    let debug = format!("{:?}", HttpHeader::new("Authorization", "Bearer secret"));
    assert!(debug.contains("Authorization"));
    assert!(!debug.contains("Bearer secret"));
}

#[test]
fn non_success_status_is_preserved_for_api_error_decoding() {
    let error = ApiEndpoint::OpenAiResponses
        .complete_with_client(
            &target(),
            "gpt-test",
            &ModelRequest::text("hello"),
            &StatusClient(429),
        )
        .unwrap_err();

    assert_eq!(error, zeta_api::ApiError::HttpStatus(429));
}

struct StatusClient(u16);

impl OperationClient for StatusClient {
    fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
        Ok(ClientResponse::new(
            self.0,
            Vec::new(),
            b"rate limited".to_vec(),
        ))
    }
}
