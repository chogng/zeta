use super::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use zeta_api::{
    Api, ApiError, HttpHeader, JsonHttpTransport, ModelRequest, StopReason, ToolDefinition,
};
use zeta_core::CoreError;
use zeta_credentials::{CredentialError, CredentialStore};

struct FixedCredentialStore(Option<String>);

impl CredentialStore for FixedCredentialStore {
    fn read_secret(&self, _: &str) -> Result<Option<String>, CredentialError> {
        Ok(self.0.clone())
    }
}

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

impl JsonHttpTransport for CapturingTransport {
    fn post_json(
        &self,
        endpoint: &str,
        headers: &[HttpHeader],
        request: Value,
    ) -> Result<Value, ApiError> {
        *self.request.lock().unwrap() = Some((endpoint.into(), headers.to_vec(), request));
        Ok(self.response.clone())
    }
}

fn provider_id(value: &str) -> ProviderId {
    ProviderId::new(value).unwrap()
}

fn model_id(value: &str) -> ModelId {
    ModelId::new(value).unwrap()
}

fn model_ref(provider: &str, model: &str) -> ModelRef {
    ModelRef::new(provider_id(provider), model_id(model))
}

fn provider_config() -> ModelProviderConfig {
    ModelProviderConfig {
        base_url: "https://example.test/v1".into(),
        credential_account: "test-account".into(),
        max_output_tokens: None,
    }
}

fn completion_response(text: &str) -> Value {
    json!({
        "id": "chatcmpl_1",
        "choices": [{
            "message": { "content": text },
            "finish_reason": "stop"
        }]
    })
}

fn responses_response(text: &str) -> Value {
    json!({
        "id": "resp_1",
        "status": "completed",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": text}]
        }]
    })
}

#[test]
fn provider_and_model_ids_reject_empty_values() {
    assert_eq!(
        ProviderId::new(" ").unwrap_err(),
        ProviderRegistryError("provider ID must not be empty".into())
    );
    assert_eq!(
        ModelId::new("").unwrap_err(),
        ProviderRegistryError("model ID must not be empty".into())
    );
}

#[test]
fn openai_provider_uses_the_responses_api() {
    let transport = Arc::new(CapturingTransport::new(responses_response(
        "Hello from OpenAI",
    )));
    let registry = ProviderRegistry::builtin_with_transport(transport.clone());
    let model = registry
        .build_model(
            &provider_config(),
            &model_ref("openai", "gpt-5.6"),
            Arc::new(FixedCredentialStore(Some("secret".into()))),
        )
        .unwrap();

    assert_eq!(model.respond("hello").unwrap(), "Hello from OpenAI");
    let (endpoint, headers, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/responses");
    assert!(
        headers
            .iter()
            .any(|header| header.name() == "Authorization" && header.value() == "Bearer secret")
    );
    assert_eq!(request["model"], "gpt-5.6");
    assert_eq!(request["input"][0]["role"], "user");
    assert_eq!(request["input"][0]["content"][0]["type"], "input_text");
    assert_eq!(request["stream"], false);
}

#[test]
fn registry_accepts_structured_tool_requests_without_an_adapter_trait() {
    let transport = Arc::new(CapturingTransport::new(json!({
        "id": "resp_1",
        "status": "completed",
        "output": [{
            "type": "function_call",
            "call_id": "call_1",
            "name": "weather",
            "arguments": "{\"city\":\"Paris\"}"
        }]
    })));
    let registry = ProviderRegistry::builtin_with_transport(transport.clone());
    let mut request = ModelRequest::text("weather");
    request.tools.push(ToolDefinition {
        name: "weather".into(),
        description: "Get weather".into(),
        parameters: json!({"type": "object"}),
        strict: true,
    });
    let response = registry
        .complete(
            &provider_config(),
            &model_ref("openai", "gpt-5.6"),
            &FixedCredentialStore(Some("secret".into())),
            &request,
        )
        .unwrap();

    assert_eq!(response.stop_reason, StopReason::ToolUse);
    assert_eq!(response.tool_calls().next().unwrap().name, "weather");
    let (_, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(body["tools"][0]["name"], "weather");
}

#[test]
fn openai_compatible_provider_uses_chat_completions() {
    let transport = Arc::new(CapturingTransport::new(completion_response(
        "Hello from compatible",
    )));
    let registry = ProviderRegistry::builtin_with_transport(transport.clone());
    let model = registry
        .build_model(
            &provider_config(),
            &model_ref("openai-compatible", "test-model"),
            Arc::new(FixedCredentialStore(Some("secret".into()))),
        )
        .unwrap();

    assert_eq!(model.respond("hello").unwrap(), "Hello from compatible");
    let (endpoint, _, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/chat/completions");
    assert_eq!(request["messages"][0]["content"], "hello");
}

#[test]
fn anthropic_provider_uses_messages_shape() {
    let transport = Arc::new(CapturingTransport::new(json!({
        "id": "msg_1",
        "content": [
            { "type": "text", "text": "Hello" },
            { "type": "text", "text": " from Anthropic" }
        ],
        "stop_reason": "end_turn"
    })));
    let registry = ProviderRegistry::builtin_with_transport(transport.clone());
    let model = registry
        .build_model(
            &ModelProviderConfig {
                base_url: String::new(),
                credential_account: "anthropic-account".into(),
                max_output_tokens: Some(2048),
            },
            &model_ref("anthropic", "claude-test"),
            Arc::new(FixedCredentialStore(Some("secret".into()))),
        )
        .unwrap();

    assert_eq!(model.respond("hello").unwrap(), "Hello from Anthropic");
    let (endpoint, headers, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://api.anthropic.com/v1/messages");
    assert!(
        headers
            .iter()
            .any(|header| header.name() == "x-api-key" && header.value() == "secret")
    );
    assert!(
        headers
            .iter()
            .any(|header| header.name() == "anthropic-version")
    );
    assert_eq!(request["model"], "claude-test");
    assert_eq!(request["max_tokens"], 2048);
}

#[test]
fn models_require_a_credential() {
    let result = ProviderRegistry::builtin().build_model(
        &provider_config(),
        &model_ref("openai", "gpt-5.6"),
        Arc::new(FixedCredentialStore(None)),
    );

    assert_eq!(
        result.err(),
        Some(CoreError::Model("model API key is missing".into()))
    );
}

#[test]
fn builtin_registry_indexes_models_under_their_provider() {
    let registry = ProviderRegistry::builtin();
    let mut provider_ids = registry
        .providers()
        .map(|provider| provider.id.as_str())
        .collect::<Vec<_>>();
    provider_ids.sort_unstable();
    assert_eq!(
        provider_ids,
        vec![
            "anthropic",
            "deepseek",
            "google",
            "huggingface",
            "kimi",
            "mimo",
            "minimax",
            "ollama",
            "openai",
            "openai-compatible",
            "qwen",
            "xai",
            "zai",
        ]
    );

    let selected = model_ref("openai", "gpt-5.6");
    assert_eq!(
        registry.get_model(&selected).unwrap().display_name,
        "GPT-5.6"
    );
    assert!(
        registry
            .get_model(&model_ref("anthropic", "gpt-5.6"))
            .is_none()
    );
    for (provider, api) in [
        ("openai", Api::OpenAi),
        ("openai-compatible", Api::OpenAiCompatible),
        ("anthropic", Api::Anthropic),
        ("google", Api::Google),
        ("xai", Api::Xai),
        ("qwen", Api::Qwen),
        ("kimi", Api::Kimi),
        ("deepseek", Api::DeepSeek),
        ("ollama", Api::Ollama),
        ("huggingface", Api::HuggingFace),
        ("zai", Api::Zai),
        ("minimax", Api::MiniMax),
        ("mimo", Api::Mimo),
    ] {
        assert_eq!(
            registry.get_provider(&provider_id(provider)).unwrap().api,
            api
        );
    }
    assert_eq!(
        registry
            .get_provider(&provider_id("openai"))
            .unwrap()
            .protocol(),
        ApiProtocol::OpenAiResponses
    );
    assert_eq!(
        registry
            .get_provider(&provider_id("anthropic"))
            .unwrap()
            .protocol(),
        ApiProtocol::AnthropicMessages
    );
}

#[test]
fn registry_rejects_an_unregistered_provider() {
    let result = ProviderRegistry::builtin().build_model(
        &provider_config(),
        &model_ref("not-registered", "test-model"),
        Arc::new(FixedCredentialStore(Some("secret".into()))),
    );
    assert_eq!(
        result.err(),
        Some(CoreError::Model(
            "model provider 'not-registered' is not registered".into()
        ))
    );
}

#[test]
fn registry_rejects_duplicate_provider_ids() {
    let mut registry = ProviderRegistry::new();
    registry.register_provider(test_provider()).unwrap();
    assert_eq!(
        registry.register_provider(test_provider()).unwrap_err(),
        ProviderRegistryError("provider ID 'test-provider' is already registered".into())
    );
}

#[test]
fn provider_rejects_duplicate_model_ids() {
    let result = Provider::new(
        provider_id("test-provider"),
        "Test Provider",
        Api::OpenAiCompatible,
        EndpointPolicy::ConfiguredOnly,
        ModelCatalogPolicy::ListedOnly,
        ProviderAuthentication::None,
    )
    .with_models([
        Model::new(model_id("same-model"), "First Model"),
        Model::new(model_id("same-model"), "Second Model"),
    ]);

    assert_eq!(
        result.unwrap_err(),
        ProviderRegistryError(
            "model ID 'same-model' is already registered under provider 'test-provider'".into()
        )
    );
}

fn test_provider() -> Provider {
    Provider::new(
        provider_id("test-provider"),
        "Test Provider",
        Api::OpenAiCompatible,
        EndpointPolicy::ConfiguredOnly,
        ModelCatalogPolicy::ListedOnly,
        ProviderAuthentication::None,
    )
    .with_models([Model::new(model_id("test-model"), "Test Model")])
    .unwrap()
}

#[test]
fn google_profile_uses_its_default_endpoint_and_fixed_header() {
    let transport = Arc::new(CapturingTransport::new(completion_response("Gemini reply")));
    let registry = ProviderRegistry::builtin_with_transport(transport.clone());
    let config = ModelProviderConfig {
        base_url: String::new(),
        credential_account: "google-api-key".into(),
        max_output_tokens: None,
    };
    let model = registry
        .build_model(
            &config,
            &model_ref("google", "gemini-3.6-flash"),
            Arc::new(FixedCredentialStore(Some("secret".into()))),
        )
        .unwrap();

    assert_eq!(model.respond("hello").unwrap(), "Gemini reply");
    let (endpoint, headers, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(
        endpoint,
        "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
    );
    assert!(
        headers
            .iter()
            .any(|header| header.name() == "x-goog-api-client" && header.value() == "zeta/0.1")
    );
}

#[test]
fn ollama_profile_uses_local_endpoint_without_authentication() {
    let transport = Arc::new(CapturingTransport::new(completion_response("Ollama reply")));
    let registry = ProviderRegistry::builtin_with_transport(transport.clone());
    let config = ModelProviderConfig {
        base_url: String::new(),
        credential_account: String::new(),
        max_output_tokens: None,
    };
    let model = registry
        .build_model(
            &config,
            &model_ref("ollama", "llama-test"),
            Arc::new(FixedCredentialStore(None)),
        )
        .unwrap();

    assert_eq!(model.respond("hello").unwrap(), "Ollama reply");
    let (endpoint, headers, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "http://localhost:11434/v1/chat/completions");
    assert!(
        !headers
            .iter()
            .any(|header| header.name() == "Authorization")
    );
}

#[test]
fn default_transport_posts_to_the_configured_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let received = Arc::new(Mutex::new(String::new()));
    let captured_request = received.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http_request(&mut stream);
        *captured_request.lock().unwrap() = request;
        let body = r#"{"id":"chatcmpl_1","choices":[{"message":{"content":"Provider reply"},"finish_reason":"stop"}]}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let registry = ProviderRegistry::builtin();
    let model = registry
        .build_model(
            &ModelProviderConfig {
                base_url: format!("http://{address}/v1"),
                credential_account: "test-account".into(),
                max_output_tokens: None,
            },
            &model_ref("openai-compatible", "test-model"),
            Arc::new(FixedCredentialStore(Some("secret".into()))),
        )
        .unwrap();

    assert_eq!(model.respond("hello").unwrap(), "Provider reply");
    server.join().unwrap();
    let request = received.lock().unwrap();
    assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
    assert!(request.contains("Authorization: Bearer secret"));
    assert!(request.contains(r#""model":"test-model""#));
    assert!(request.contains(r#""content":"hello""#));
}

fn read_http_request(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = stream.read(&mut buffer).unwrap();
        assert_ne!(bytes_read, 0, "request ended before its headers");
        request.extend_from_slice(&buffer[..bytes_read]);
        let Some(headers_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = std::str::from_utf8(&request[..headers_end]).unwrap();
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .unwrap()
            .parse::<usize>()
            .unwrap();
        if request.len() >= headers_end + 4 + content_length {
            return String::from_utf8(request).unwrap();
        }
    }
}
