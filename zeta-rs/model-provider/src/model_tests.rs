use super::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use zeta_api::{ModelRequest, ModelStreamEvent, StopReason, ToolDefinition, ToolName};
use zeta_async_utils::CancellationSource;
use zeta_client::{
    ClientError, ClientRequest, ClientResponse, OperationClient, OperationStreamSink,
};
use zeta_context_engine::ContextTokenMeasurementAccuracy;
use zeta_context_engine::ContextTokenMeasurementCapability;
use zeta_context_engine::ContextTokenMeasurementOutcome;
use zeta_http_client::HttpHeader;
use zeta_model_provider_config::{
    ApiProfile, EndpointPolicy, ModelCatalogPolicy, ModelProviderConfig, ProviderAdapter,
    ProviderConfigError, ProviderConfigRegistry, ProviderDefinition,
};
use zeta_model_tokenizer::LocalTokenCount;
use zeta_model_tokenizer::LocalTokenizationOutcome;
use zeta_model_tokenizer::LocalTokenizerError;
use zeta_model_tokenizer::LocalTokenizerService;

#[test]
fn api_failure_categories_are_preserved_by_the_provider_boundary() {
    assert_eq!(
        ModelProviderError::from(ApiError::ContextOverflow("context detail".into())),
        ModelProviderError::ContextOverflow("context detail".into())
    );
    assert_eq!(
        ModelProviderError::from(ApiError::AuthFailed("auth detail".into())),
        ModelProviderError::AuthFailed("auth detail".into())
    );
    assert_eq!(
        ModelProviderError::from(ApiError::InvalidRequest("request detail".into())),
        ModelProviderError::InvalidRequest("request detail".into())
    );
    assert_eq!(
        ModelProviderError::from(ApiError::InvalidResponse("response detail".into())),
        ModelProviderError::InvalidResponse("response detail".into())
    );
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

struct FailingTransport;

impl OperationClient for FailingTransport {
    fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
        Err(ClientError::Transport(
            "fixture count endpoint failure".into(),
        ))
    }
}

struct StreamingTransport;

impl OperationClient for StreamingTransport {
    fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
        panic!("registered Responses models must retain the streaming operation path")
    }

    fn execute_streaming(
        &self,
        _: &ClientRequest,
        sink: &mut dyn OperationStreamSink,
    ) -> Result<ClientResponse, ClientError> {
        let payload = concat!(
            "event: response.output_text.delta\n",
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"live\"}\n\n",
            "event: response.completed\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"live\"}]}]}}\n\n",
        );
        sink.emit(payload.as_bytes())?;
        Ok(ClientResponse::new(200, Vec::new(), Vec::new()))
    }
}

#[derive(Default)]
struct RecordedModelEvents(Vec<ModelStreamEvent>);

impl ModelEventSink for RecordedModelEvents {
    fn emit(&mut self, event: ModelStreamEvent) -> Result<(), ModelProviderError> {
        self.0.push(event);
        Ok(())
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

fn provider_config(provider: &str) -> ModelProviderConfig {
    ModelProviderConfig::new(provider_id(provider))
}

fn provider_config_with_endpoint(
    provider: &str,
    base_url: impl Into<String>,
) -> ModelProviderConfig {
    ModelProviderConfig {
        provider: provider_id(provider),
        base_url: Some(base_url.into()),
        max_output_tokens: None,
        model_context: Default::default(),
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

fn invoke_text(model: &dyn ModelInvoker, prompt: &str) -> String {
    model.invoke(&ModelRequest::text(prompt)).unwrap().text()
}

#[test]
fn registered_model_propagates_cancellation_to_the_operation_client() {
    let transport = Arc::new(CancellationRecordingTransport::default());
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(&provider_config("openai"), &model_ref("openai", "gpt-5.6"))
        .unwrap();

    let result = model.invoke_with_cancellation(
        &ModelRequest::text("hello"),
        &CancellationSource::new().token(),
    );

    assert_eq!(
        result,
        Err(ModelProviderError::Cancelled(
            "cancelled inside operation client".into()
        ))
    );
    assert!(transport.cancellable_path.load(Ordering::Relaxed));
}

#[test]
fn registered_openai_model_propagates_wire_stream_events() {
    let runtime = ModelProviderRuntime::builtin_with_client(Arc::new(StreamingTransport));
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("openai", "https://example.test/v1"),
            &model_ref("openai", "gpt-5.6"),
        )
        .unwrap();
    let mut events = RecordedModelEvents::default();

    let response = model
        .stream_with_cancellation(
            &ModelRequest::text("hello"),
            &CancellationSource::new().token(),
            &mut events,
        )
        .unwrap();

    assert_eq!(events.0, vec![ModelStreamEvent::TextDelta("live".into())]);
    assert_eq!(response.text(), "live");
}

#[test]
fn provider_and_model_ids_reject_empty_values() {
    assert_eq!(
        ProviderId::new(" ").unwrap_err().to_string(),
        "provider ID must not be empty"
    );
    assert_eq!(
        ModelId::new("").unwrap_err().to_string(),
        "model ID must not be empty"
    );
}

#[derive(Default)]
struct CancellationRecordingTransport {
    cancellable_path: AtomicBool,
}

impl OperationClient for CancellationRecordingTransport {
    fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
        panic!("registered models must use the cancellable operation path")
    }

    fn execute_with_cancellation(
        &self,
        _: &ClientRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<ClientResponse, ClientError> {
        self.cancellable_path.store(true, Ordering::Relaxed);
        Err(ClientError::Cancelled(
            "cancelled inside operation client".into(),
        ))
    }
}

#[test]
fn openai_runtime_uses_the_responses_adapter_and_dynamic_endpoint() {
    let transport = Arc::new(CapturingTransport::new(responses_response(
        "Hello from OpenAI",
    )));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("openai", " https://example.test/v1/ "),
            &model_ref("openai", "gpt-5.6"),
        )
        .unwrap();

    assert_eq!(invoke_text(model.as_ref(), "hello"), "Hello from OpenAI");
    let (endpoint, headers, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/responses");
    assert!(
        headers
            .iter()
            .all(|header| header.name() != "Authorization")
    );
    assert_eq!(request["model"], "gpt-5.6");
    assert_eq!(request["input"][0]["role"], "user");
    assert_eq!(request["input"][0]["content"][0]["type"], "input_text");
}

#[test]
fn openai_runtime_exposes_exact_remote_input_measurement() {
    let transport = Arc::new(CapturingTransport::new(json!({"input_tokens": 321})));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("openai", "https://example.test/v1"),
            &model_ref("openai", "gpt-5.6"),
        )
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Remote
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("expected a provider measurement");
    };
    assert_eq!(measurement.measured_input().get(), 321);
    assert_eq!(measurement.accounted_input().get(), 321);
    assert_eq!(
        measurement.accuracy(),
        ContextTokenMeasurementAccuracy::Exact
    );
    let (endpoint, _, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/responses/input_tokens");
}

#[test]
fn provider_preflight_takes_priority_over_an_available_local_counter() {
    let transport = Arc::new(CapturingTransport::new(json!({"input_tokens": 321})));
    let local_tokenizers = Arc::new(FixedLocalTokenizer {
        model: model_ref("openai", "gpt-5.6"),
        tokens: 99,
    });
    let runtime = ModelProviderRuntime::builtin_with_client(transport)
        .with_local_tokenizers(local_tokenizers);
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("openai", "https://example.test/v1"),
            &model_ref("openai", "gpt-5.6"),
        )
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Remote
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("expected the provider preflight result");
    };
    assert_eq!(measurement.measured_input().get(), 321);
    assert_eq!(
        measurement.source().kind(),
        zeta_context_engine::ContextTokenMeasurementSourceKind::ProviderPreflight
    );
}

#[test]
fn provider_preflight_failure_falls_back_to_the_local_counter() {
    let local_tokenizers = Arc::new(FixedLocalTokenizer {
        model: model_ref("openai", "gpt-5.6"),
        tokens: 99,
    });
    let runtime = ModelProviderRuntime::builtin_with_client(Arc::new(FailingTransport))
        .with_local_tokenizers(local_tokenizers);
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("openai", "https://example.test/v1"),
            &model_ref("openai", "gpt-5.6"),
        )
        .unwrap();

    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("the local tokenizer should cover provider preflight failures");
    };
    assert_eq!(measurement.measured_input().get(), 99);
    assert_eq!(
        measurement.source().kind(),
        zeta_context_engine::ContextTokenMeasurementSourceKind::LocalTokenizer
    );
}

#[test]
fn model_provider_resolves_runtime_from_declarative_config() {
    let transport = Arc::new(CapturingTransport::new(completion_response(
        "Unified runtime",
    )));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model_provider: &dyn ModelProvider = &runtime;

    let model = model_provider
        .runtime(ModelRuntimeRequest::new(
            model_ref("deepseek", "deepseek-v4-pro"),
            provider_config_with_endpoint("deepseek", "https://example.test/v1"),
        ))
        .unwrap();

    assert_eq!(invoke_text(model.as_ref(), "hello"), "Unified runtime");
    let (endpoint, _, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/chat/completions");
    assert_eq!(request["model"], "deepseek-v4-pro");
}

#[test]
fn deepseek_uses_only_an_exact_model_binding_for_local_measurement() {
    let transport = Arc::new(CapturingTransport::new(completion_response("unused")));
    let local_tokenizers = Arc::new(FixedLocalTokenizer {
        model: model_ref("deepseek", "deepseek-chat"),
        tokens: 120,
    });
    let runtime = ModelProviderRuntime::builtin_with_client(transport)
        .with_local_tokenizers(local_tokenizers);
    let bound = runtime
        .build_model(
            &provider_config("deepseek"),
            &model_ref("deepseek", "deepseek-chat"),
        )
        .unwrap();

    assert_eq!(
        bound.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Local
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        bound.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("bound DeepSeek model should use the local tokenizer");
    };
    assert_eq!(measurement.measured_input().get(), 120);
    assert_eq!(measurement.accounted_input().get(), 184);
    assert_eq!(
        measurement.accuracy(),
        ContextTokenMeasurementAccuracy::Estimated
    );
    assert_eq!(
        measurement.source().kind(),
        zeta_context_engine::ContextTokenMeasurementSourceKind::LocalTokenizer
    );

    let unbound = runtime
        .build_model(
            &provider_config("deepseek"),
            &model_ref("deepseek", "deepseek-reasoner"),
        )
        .unwrap();
    assert_eq!(
        unbound.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Unavailable
    );
}

#[test]
fn huggingface_uses_the_same_model_bound_local_tokenizer_port() {
    let transport = Arc::new(CapturingTransport::new(completion_response("unused")));
    let local_tokenizers = Arc::new(FixedLocalTokenizer {
        model: model_ref("huggingface", "org/model"),
        tokens: 80,
    });
    let runtime = ModelProviderRuntime::builtin_with_client(transport)
        .with_local_tokenizers(local_tokenizers);
    let model = runtime
        .build_model(
            &provider_config("huggingface"),
            &model_ref("huggingface", "org/model"),
        )
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Local
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("bound Hugging Face model should use the local tokenizer");
    };
    assert_eq!(measurement.measured_input().get(), 80);
    assert_eq!(measurement.accounted_input().get(), 144);
    assert_eq!(
        measurement.accuracy(),
        ContextTokenMeasurementAccuracy::Estimated
    );
}

struct FixedLocalTokenizer {
    model: ModelRef,
    tokens: u32,
}

impl LocalTokenizerService for FixedLocalTokenizer {
    fn supports(&self, model: &ModelRef) -> bool {
        model == &self.model
    }

    fn count_input_tokens(
        &self,
        model: &ModelRef,
        _: &ModelRequest,
    ) -> Result<LocalTokenizationOutcome, LocalTokenizerError> {
        if !self.supports(model) {
            return Ok(LocalTokenizationOutcome::UnsupportedRequest);
        }
        Ok(LocalTokenizationOutcome::Count(LocalTokenCount::new(
            self.tokens,
            "fixture-tokenizer-and-template-revision",
        )?))
    }
}

#[test]
fn runtime_accepts_structured_tool_requests() {
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
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let mut request = ModelRequest::text("weather");
    request.tools.push(ToolDefinition {
        name: ToolName::new("weather").expect("test tool name is valid"),
        description: "Get weather".into(),
        parameters: json!({"type": "object"}),
        strict: true,
    });
    let response = runtime
        .complete(
            &provider_config("openai"),
            &model_ref("openai", "gpt-5.6"),
            &request,
        )
        .unwrap();

    assert_eq!(response.stop_reason, StopReason::ToolUse);
    assert_eq!(
        response.tool_calls().next().unwrap().name.as_str(),
        "weather"
    );
    let (_, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(body["tools"][0]["name"], "weather");
}

#[test]
fn openai_compatible_requires_a_configured_endpoint() {
    let runtime = ModelProviderRuntime::builtin();
    assert_eq!(
        runtime
            .instantiate(&provider_config("openai-compatible"))
            .err()
            .unwrap(),
        ModelProviderError::Config(ProviderConfigError::MissingBaseUrl(provider_id(
            "openai-compatible"
        )))
    );
}

#[test]
fn anthropic_runtime_uses_messages_shape_and_declarative_defaults() {
    let transport = Arc::new(CapturingTransport::new(json!({
        "id": "msg_1",
        "content": [
            { "type": "text", "text": "Hello" },
            { "type": "text", "text": " from Anthropic" }
        ],
        "stop_reason": "end_turn"
    })));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config("anthropic"),
            &model_ref("anthropic", "claude-test"),
        )
        .unwrap();

    assert_eq!(invoke_text(model.as_ref(), "hello"), "Hello from Anthropic");
    let (endpoint, headers, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://api.anthropic.com/v1/messages");
    assert!(
        headers
            .iter()
            .any(|header| header.name() == "anthropic-version")
    );
    assert!(headers.iter().all(|header| header.name() != "x-api-key"));
    assert_eq!(request["model"], "claude-test");
    assert_eq!(request["max_tokens"], 1024);
}

#[test]
fn anthropic_runtime_exposes_conservative_remote_input_measurement() {
    let transport = Arc::new(CapturingTransport::new(json!({"input_tokens": 10_000})));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config("anthropic"),
            &model_ref("anthropic", "claude-test"),
        )
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Remote
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("expected a provider measurement");
    };
    assert_eq!(measurement.measured_input().get(), 10_000);
    assert_eq!(measurement.accounted_input().get(), 10_100);
    assert_eq!(
        measurement.accuracy(),
        ContextTokenMeasurementAccuracy::Estimated
    );
    let (endpoint, _, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(
        endpoint,
        "https://api.anthropic.com/v1/messages/count_tokens"
    );
}

#[test]
fn google_runtime_uses_native_count_tokens_as_a_conservative_measurement() {
    let transport = Arc::new(CapturingTransport::new(json!({"totalTokens": 100})));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config("google"),
            &model_ref("google", "gemini-3.6-flash"),
        )
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Remote
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("expected a provider measurement");
    };
    assert_eq!(measurement.measured_input().get(), 100);
    assert_eq!(measurement.accounted_input().get(), 132);
    assert_eq!(
        measurement.accuracy(),
        ContextTokenMeasurementAccuracy::Estimated
    );
    let (endpoint, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(
        endpoint,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:countTokens"
    );
    assert_eq!(
        body["generateContentRequest"]["model"],
        "models/gemini-3.6-flash"
    );
}

#[test]
fn provider_measurement_capability_is_model_specific() {
    let runtime = ModelProviderRuntime::builtin();
    let google_model = runtime
        .build_model(
            &provider_config("google"),
            &model_ref("google", "unlisted-gemini-model"),
        )
        .unwrap();
    let kimi_model = runtime
        .build_model(
            &provider_config("kimi"),
            &model_ref("kimi", "unlisted-kimi-model"),
        )
        .unwrap();
    let zai_model = runtime
        .build_model(
            &provider_config("zai"),
            &model_ref("zai", "unlisted-glm-model"),
        )
        .unwrap();

    assert_eq!(
        google_model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Unavailable
    );
    assert_eq!(
        kimi_model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Unavailable
    );
    assert_eq!(
        zai_model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Unavailable
    );
}

#[test]
fn google_custom_invocation_endpoint_does_not_guess_a_native_count_url() {
    let runtime = ModelProviderRuntime::builtin();
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("google", "https://proxy.test/v1/openai"),
            &model_ref("google", "gemini-3.6-flash"),
        )
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Unavailable
    );
}

#[test]
fn kimi_runtime_exposes_the_documented_remote_estimate() {
    let transport = Arc::new(CapturingTransport::new(
        json!({"data": {"total_tokens": 200}}),
    ));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(&provider_config("kimi"), &model_ref("kimi", "kimi-k2.6"))
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Remote
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("expected a provider measurement");
    };
    assert_eq!(measurement.measured_input().get(), 200);
    assert_eq!(measurement.accounted_input().get(), 232);
    let (endpoint, _, body) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(
        endpoint,
        "https://api.moonshot.ai/v1/tokenizers/estimate-token-count"
    );
    assert_eq!(body["messages"][0]["content"], "hello");
}

#[test]
fn zai_runtime_exposes_the_documented_remote_tokenizer() {
    let transport = Arc::new(CapturingTransport::new(
        json!({"usage": {"prompt_tokens": 300, "total_tokens": 300}}),
    ));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(&provider_config("zai"), &model_ref("zai", "glm-5.1"))
        .unwrap();

    assert_eq!(
        model.input_token_measurement_capability(),
        ContextTokenMeasurementCapability::Remote
    );
    let ContextTokenMeasurementOutcome::Measured(measurement) =
        model.measure_input(&ModelRequest::text("hello")).unwrap()
    else {
        panic!("expected a provider measurement");
    };
    assert_eq!(measurement.measured_input().get(), 300);
    assert_eq!(measurement.accounted_input().get(), 332);
    let (endpoint, _, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://api.z.ai/api/paas/v4/tokenizer");
}

#[test]
fn request_output_limit_overrides_the_provider_default() {
    let transport = Arc::new(CapturingTransport::new(json!({
        "id": "msg_1",
        "content": [{ "type": "text", "text": "Compacted" }],
        "stop_reason": "end_turn"
    })));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config("anthropic"),
            &model_ref("anthropic", "claude-test"),
        )
        .unwrap();
    let mut request = ModelRequest::text("compact this context");
    request.max_output_tokens = Some(128);

    assert_eq!(model.invoke(&request).unwrap().text(), "Compacted");
    let (_, _, request) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(request["max_tokens"], 128);
}

#[test]
fn builtin_runtime_instantiates_provider_protocols() {
    let runtime = ModelProviderRuntime::builtin();
    assert_eq!(
        runtime
            .instantiate(&provider_config("openai"))
            .unwrap()
            .protocol(),
        ApiProtocol::OpenAiResponses
    );
    assert_eq!(
        runtime
            .instantiate(&provider_config("anthropic"))
            .unwrap()
            .protocol(),
        ApiProtocol::AnthropicMessages
    );
}

#[test]
fn final_image_detail_gate_uses_model_capability_not_protocol_family() {
    let openai_transport = Arc::new(CapturingTransport::new(responses_response("ok")));
    let openai_runtime = ModelProviderRuntime::builtin_with_client(openai_transport.clone());
    let openai_model = openai_runtime
        .build_model(&provider_config("openai"), &model_ref("openai", "gpt-5.6"))
        .unwrap();
    let request = request_with_original_image();
    openai_model.invoke(&request).unwrap();
    let (_, _, body) = openai_transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(body["input"][0]["content"][0]["detail"], "original");

    let custom_definition = ProviderDefinition::new(
        provider_id("custom-responses"),
        "Custom Responses",
        ProviderAdapter::OpenAiCompatible,
        ApiProfile::OpenAiResponses,
        EndpointPolicy::ConfiguredOnly,
        ModelCatalogPolicy::AllowUnlisted,
    );
    let custom_transport = Arc::new(CapturingTransport::new(responses_response("ok")));
    let custom_runtime = ModelProviderRuntime::with_client(
        ProviderConfigRegistry::from_definitions([custom_definition]).unwrap(),
        custom_transport.clone(),
    );
    let custom_model = custom_runtime
        .build_model(
            &provider_config_with_endpoint("custom-responses", "https://example.test/v1"),
            &model_ref("custom-responses", "unknown-model"),
        )
        .unwrap();
    custom_model.invoke(&request).unwrap();
    let (_, _, body) = custom_transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(body["input"][0]["content"][0]["detail"], "auto");
}

fn request_with_original_image() -> ModelRequest {
    ModelRequest {
        instructions: None,
        input: vec![zeta_api::InputItem::Message(zeta_api::Message {
            role: zeta_api::MessageRole::User,
            content: vec![zeta_api::ContentPart::ImageUrl {
                url: "data:image/png;base64,AA==".into(),
                detail: zeta_api::ImageDetail::Original,
            }],
            tool_calls: Vec::new(),
        })],
        tools: Vec::new(),
        tool_choice: zeta_api::ToolChoice::None,
        parallel_tool_calls: false,
        reasoning: None,
        max_output_tokens: None,
        temperature: None,
    }
}

#[test]
fn runtime_reports_unknown_and_mismatched_providers_as_config_errors() {
    let runtime = ModelProviderRuntime::builtin();
    assert_eq!(
        runtime
            .build_model(
                &provider_config("not-registered"),
                &model_ref("not-registered", "test-model"),
            )
            .err()
            .unwrap(),
        ModelProviderError::Config(ProviderConfigError::UnknownProvider(provider_id(
            "not-registered"
        )))
    );
    assert_eq!(
        runtime
            .build_model(
                &provider_config("openai"),
                &model_ref("anthropic", "claude-test"),
            )
            .err()
            .unwrap(),
        ModelProviderError::Config(ProviderConfigError::ProviderMismatch {
            configured: provider_id("openai"),
            selected: provider_id("anthropic"),
        })
    );
}

#[test]
fn listed_catalog_rejects_unregistered_models_at_runtime() {
    let definition = ProviderDefinition::new(
        provider_id("test-provider"),
        "Test Provider",
        ProviderAdapter::OpenAiCompatible,
        ApiProfile::OpenAiChatCompletions,
        EndpointPolicy::ConfiguredOnly,
        ModelCatalogPolicy::ListedOnly,
    )
    .with_models([Model::new(model_id("listed"), "Listed")]);
    let configs = ProviderConfigRegistry::from_definitions([definition]).unwrap();
    let runtime = ModelProviderRuntime::new(configs);
    let error = runtime
        .build_model(
            &provider_config_with_endpoint("test-provider", "https://example.test/v1"),
            &model_ref("test-provider", "unlisted"),
        )
        .err()
        .unwrap();

    assert_eq!(
        error,
        ModelProviderError::ModelNotRegistered {
            provider: provider_id("test-provider"),
            model: model_id("unlisted"),
        }
    );
}

#[test]
fn runtime_uses_the_declarative_api_profile_instead_of_the_adapter_name() {
    let definition = ProviderDefinition::new(
        provider_id("profile-test"),
        "Profile Test",
        ProviderAdapter::OpenAiCompatible,
        ApiProfile::OpenAiResponses,
        EndpointPolicy::ConfiguredOnly,
        ModelCatalogPolicy::AllowUnlisted,
    );
    let transport = Arc::new(CapturingTransport::new(responses_response(
        "profile selected",
    )));
    let runtime = ModelProviderRuntime::with_client(
        ProviderConfigRegistry::from_definitions([definition]).unwrap(),
        transport.clone(),
    );
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("profile-test", "https://example.test/v1"),
            &model_ref("profile-test", "test-model"),
        )
        .unwrap();

    assert_eq!(invoke_text(model.as_ref(), "hello"), "profile selected");
    let (endpoint, _, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "https://example.test/v1/responses");
    assert_eq!(
        runtime
            .instantiate(&provider_config_with_endpoint(
                "profile-test",
                "https://example.test/v1",
            ))
            .unwrap()
            .protocol(),
        ApiProtocol::OpenAiResponses
    );
}

#[test]
fn google_runtime_adds_its_fixed_header() {
    let transport = Arc::new(CapturingTransport::new(completion_response("Gemini reply")));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config("google"),
            &model_ref("google", "gemini-3.6-flash"),
        )
        .unwrap();

    assert_eq!(invoke_text(model.as_ref(), "hello"), "Gemini reply");
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
fn ollama_runtime_uses_its_local_default_endpoint() {
    let transport = Arc::new(CapturingTransport::new(completion_response("Ollama reply")));
    let runtime = ModelProviderRuntime::builtin_with_client(transport.clone());
    let model = runtime
        .build_model(
            &provider_config("ollama"),
            &model_ref("ollama", "llama-test"),
        )
        .unwrap();

    assert_eq!(invoke_text(model.as_ref(), "hello"), "Ollama reply");
    let (endpoint, headers, _) = transport.request.lock().unwrap().clone().unwrap();
    assert_eq!(endpoint, "http://localhost:11434/v1/chat/completions");
    assert!(headers.is_empty());
}

#[test]
fn default_transport_posts_to_the_normalized_endpoint() {
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
    let runtime = ModelProviderRuntime::builtin();
    let model = runtime
        .build_model(
            &provider_config_with_endpoint("openai-compatible", format!("http://{address}/v1/")),
            &model_ref("openai-compatible", "test-model"),
        )
        .unwrap();

    assert_eq!(invoke_text(model.as_ref(), "hello"), "Provider reply");
    server.join().unwrap();
    let request = received.lock().unwrap();
    assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
    assert!(!request.contains("Authorization:"));
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
