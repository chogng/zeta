use serde_json::{Value, json};
use std::sync::Mutex;
use zeta_api::{
    ApiEndpoint, ApiProtocol, InputTokenCountEndpoint, ModelRequest, ReasoningConfig,
    ReasoningEffort, SemanticApiEndpoint, StopReason, ToolDefinition, ToolName,
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
fn openai_responses_counts_the_frozen_input_payload() {
    let transport = CapturingTransport::new(json!({"input_tokens": 321}));
    let mut request = tool_request();
    request.max_output_tokens = Some(2_048);
    request.temperature = Some(0.25);

    let count = ApiEndpoint::OpenAiResponses
        .count_input_tokens_with_client(&target(), "gpt-test", &request, &transport)
        .unwrap();

    let (endpoint, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/responses/input_tokens");
    assert_eq!(count.get(), 321);
    assert_eq!(body["model"], "gpt-test");
    assert_eq!(body["tools"][0]["name"], "weather");
    assert_eq!(body["reasoning"]["effort"], "medium");
    assert_eq!(body["tool_choice"], "auto");
    assert_eq!(body["parallel_tool_calls"], true);
    assert!(body.get("stream").is_none());
    assert!(body.get("max_output_tokens").is_none());
    assert!(body.get("temperature").is_none());
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
fn anthropic_messages_counts_the_frozen_input_payload() {
    let transport = CapturingTransport::new(json!({"input_tokens": 144}));
    let mut request = tool_request();
    request.reasoning = None;
    request.max_output_tokens = Some(2_048);
    request.temperature = Some(0.25);
    let target = ResolvedApiTarget::new(
        "https://api.anthropic.com",
        vec![HttpHeader::new("x-api-key", "secret")],
    );

    let count = ApiEndpoint::AnthropicMessages
        .count_input_tokens_with_client(&target, "claude-test", &request, &transport)
        .unwrap();

    let (endpoint, headers, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(
        endpoint,
        "https://api.anthropic.com/v1/messages/count_tokens"
    );
    assert_eq!(count.get(), 144);
    assert_eq!(body["model"], "claude-test");
    assert_eq!(body["tools"][0]["name"], "weather");
    assert_eq!(body["tool_choice"]["type"], "auto");
    assert!(body.get("max_tokens").is_none());
    assert!(body.get("temperature").is_none());
    assert!(
        headers
            .iter()
            .any(|header| { header.name().eq_ignore_ascii_case("anthropic-version") })
    );
}

#[test]
fn gemini_count_tokens_encodes_the_native_generate_content_request() {
    let transport = CapturingTransport::new(json!({"totalTokens": 233}));
    let mut request = tool_request();
    request.reasoning = None;
    let target = ResolvedApiTarget::new(
        "https://generativelanguage.googleapis.com/v1beta",
        vec![HttpHeader::new("x-goog-api-client", "zeta/0.1")],
    );

    let count = InputTokenCountEndpoint::GoogleGenerateContent
        .count_with_client(&target, "gemini-test", &request, &transport)
        .unwrap();

    let (endpoint, _, body) = transport.request.lock().unwrap().clone().unwrap();
    let generate = &body["generateContentRequest"];
    assert_eq!(
        endpoint,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-test:countTokens"
    );
    assert_eq!(count.get(), 233);
    assert_eq!(generate["model"], "models/gemini-test");
    assert_eq!(
        generate["systemInstruction"]["parts"][0]["text"],
        "Use tools when needed."
    );
    assert_eq!(generate["contents"][0]["role"], "user");
    assert_eq!(
        generate["tools"][0]["functionDeclarations"][0]["name"],
        "weather"
    );
    assert_eq!(
        generate["toolConfig"]["functionCallingConfig"]["mode"],
        "AUTO"
    );
}

#[test]
fn kimi_estimate_tokens_sends_only_the_documented_input_fields() {
    let transport = CapturingTransport::new(json!({"data": {"total_tokens": 87}}));
    let mut request = ModelRequest::text("hello");
    request.max_output_tokens = Some(512);
    request.temperature = Some(0.5);

    let count = InputTokenCountEndpoint::KimiChatCompletions
        .count_with_client(&target(), "kimi-k2.6", &request, &transport)
        .unwrap();

    let (endpoint, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(
        endpoint,
        "https://example.test/v1/tokenizers/estimate-token-count"
    );
    assert_eq!(count.get(), 87);
    assert_eq!(body["model"], "kimi-k2.6");
    assert_eq!(body["messages"][0]["content"], "hello");
    assert!(body.get("stream").is_none());
    assert!(body.get("max_tokens").is_none());
    assert!(body.get("temperature").is_none());
}

#[test]
fn zai_tokenizer_preserves_tools_but_removes_generation_fields() {
    let transport = CapturingTransport::new(json!({
        "usage": {
            "prompt_tokens": 120,
            "image_tokens": 24,
            "total_tokens": 144
        }
    }));
    let mut request = tool_request();
    request.reasoning = None;
    request.max_output_tokens = Some(512);

    let count = InputTokenCountEndpoint::ZaiChatCompletions
        .count_with_client(&target(), "glm-5.1", &request, &transport)
        .unwrap();

    let (endpoint, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/tokenizer");
    assert_eq!(count.get(), 144);
    assert_eq!(body["tools"][0]["function"]["name"], "weather");
    assert!(body["tools"][0]["function"].get("strict").is_none());
    assert!(body.get("tool_choice").is_none());
    assert!(body.get("max_tokens").is_none());
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
fn semantic_endpoints_restore_provider_results_to_input_order() {
    let embeddings = CapturingTransport::new(json!({
        "data": [
            {"index": 1, "embedding": [0.0, 1.0]},
            {"index": 0, "embedding": [1.0, 0.0]}
        ]
    }));
    let vectors = SemanticApiEndpoint::OpenAiCompatible
        .embed_with_client(
            &target(),
            "embed-v1",
            &["first".into(), "second".into()],
            &embeddings,
        )
        .unwrap();
    assert_eq!(vectors, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    let (_, headers, body) = embeddings.request.lock().unwrap().clone().unwrap();
    assert_eq!(body["input"], json!(["first", "second"]));
    assert!(headers.iter().any(|header| {
        header.name().eq_ignore_ascii_case("content-type") && header.value() == "application/json"
    }));

    let rerank = CapturingTransport::new(json!({
        "results": [
            {"index": 1, "relevance_score": 0.8},
            {"index": 0, "score": 0.2}
        ]
    }));
    let scores = SemanticApiEndpoint::OpenAiCompatible
        .rerank_with_client(
            &target(),
            "rerank-v1",
            "query",
            &["first".into(), "second".into()],
            &rerank,
        )
        .unwrap();
    assert_eq!(scores, vec![0.2, 0.8]);
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

    assert_eq!(
        error,
        zeta_api::ApiError::RateLimited {
            retry_after_ms: None
        }
    );
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
