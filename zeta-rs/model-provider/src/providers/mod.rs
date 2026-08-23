use crate::ModelProviderError;
use crate::provider::ModelEventSink;
use std::sync::Arc;
use zeta_api::ApiEndpoint;
use zeta_api::ApiProtocol;
use zeta_api::ApiStreamSink;
use zeta_api::ModelRequest;
use zeta_api::ModelResponse;
use zeta_api::ModelStreamEvent;
use zeta_async_utils::CancellationToken;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_context_engine::ContextTokenMeasurementCapability;
use zeta_context_engine::ContextTokenMeasurementOutcome;
use zeta_model_provider_config::{
    ApiProfile, NormalizedModelProviderConfig, ProviderAdapter as ProviderAdapterKind,
};

mod anthropic;
mod deepseek;
mod google;
mod huggingface;
mod kimi;
pub(crate) mod measurement;
mod mimo;
mod minimax;
mod ollama;
mod openai;
mod openai_compatible;
mod qwen;
mod xai;
mod zai;

/// Converts one normalized provider configuration into provider-specific API requests.
///
/// Implementations own the resolved endpoint and fixed provider headers for one immutable runtime
/// snapshot. They delegate wire encoding to `zeta-api` and must return provider errors without
/// exposing transport-specific state to callers.
pub(crate) trait ProviderAdapter: Send + Sync {
    fn protocol(&self) -> ApiProtocol;

    /// Reports whether this immutable adapter can measure canonical input locally or remotely.
    fn input_token_measurement_capability(&self, _: &str) -> ContextTokenMeasurementCapability {
        ContextTokenMeasurementCapability::Unavailable
    }

    /// Measures one fully assembled request using the adapter's declared tokenizer contract.
    fn measure_input(
        &self,
        _: &str,
        _: &ModelRequest,
        _: &dyn OperationClient,
        _: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, ModelProviderError> {
        Ok(ContextTokenMeasurementOutcome::Unavailable)
    }

    fn complete(
        &self,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError>;

    fn stream(
        &self,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelProviderError> {
        let response = self.complete(model, request, client, cancellation)?;
        emit_final_response(&response, sink)?;
        Ok(response)
    }
}

pub(crate) fn stream_endpoint(
    endpoint: ApiEndpoint,
    target: &zeta_client::ResolvedApiTarget,
    model: &str,
    request: &ModelRequest,
    client: &dyn OperationClient,
    cancellation: &CancellationToken,
    sink: &mut dyn ModelEventSink,
) -> Result<ModelResponse, ModelProviderError> {
    let mut sink = ProviderApiStreamSink {
        inner: sink,
        failure: None,
    };
    let response = endpoint.stream_with_client_and_cancellation(
        target,
        model,
        request,
        client,
        cancellation,
        &mut sink,
    );
    if let Some(error) = sink.failure {
        return Err(error);
    }
    response.map_err(Into::into)
}

struct ProviderApiStreamSink<'a> {
    inner: &'a mut dyn ModelEventSink,
    failure: Option<ModelProviderError>,
}

impl ApiStreamSink for ProviderApiStreamSink<'_> {
    fn emit(&mut self, event: ModelStreamEvent) -> Result<(), zeta_api::ApiError> {
        if let Err(error) = self.inner.emit(event) {
            self.failure = Some(error);
            return Err(zeta_api::ApiError::Transport(
                "model stream consumer rejected an event".into(),
            ));
        }
        Ok(())
    }
}

fn emit_final_response(
    response: &ModelResponse,
    sink: &mut dyn ModelEventSink,
) -> Result<(), ModelProviderError> {
    for item in &response.output {
        let event = match item {
            zeta_api::OutputItem::Text(text) => Some(ModelStreamEvent::TextDelta(text.clone())),
            zeta_api::OutputItem::Reasoning(text) => {
                Some(ModelStreamEvent::ReasoningDelta(text.clone()))
            }
            zeta_api::OutputItem::Refusal(_) | zeta_api::OutputItem::ToolCall(_) => None,
        };
        if let Some(event) = event {
            sink.emit(event)?;
        }
    }
    Ok(())
}

pub(crate) fn instantiate(
    adapter: ProviderAdapterKind,
    config: &NormalizedModelProviderConfig,
    target_override: Option<ResolvedApiTarget>,
) -> Arc<dyn ProviderAdapter> {
    match adapter {
        ProviderAdapterKind::OpenAi => runtime_adapter(match target_override {
            Some(target) => openai::OpenAiAdapter::with_target(config, target),
            None => openai::OpenAiAdapter::new(config),
        }),
        ProviderAdapterKind::OpenAiCompatible => {
            runtime_adapter(openai_compatible::OpenAiCompatibleAdapter::new(config))
        }
        ProviderAdapterKind::Anthropic => runtime_adapter(anthropic::AnthropicAdapter::new(config)),
        ProviderAdapterKind::Google => runtime_adapter(google::GoogleAdapter::new(config)),
        ProviderAdapterKind::Xai => runtime_adapter(xai::XaiAdapter::new(config)),
        ProviderAdapterKind::Qwen => runtime_adapter(qwen::QwenAdapter::new(config)),
        ProviderAdapterKind::Kimi => runtime_adapter(match target_override {
            Some(target) => kimi::KimiAdapter::with_target(config, target),
            None => kimi::KimiAdapter::new(config),
        }),
        ProviderAdapterKind::DeepSeek => runtime_adapter(deepseek::DeepSeekAdapter::new(config)),
        ProviderAdapterKind::Ollama => runtime_adapter(ollama::OllamaAdapter::new(config)),
        ProviderAdapterKind::HuggingFace => {
            runtime_adapter(huggingface::HuggingFaceAdapter::new(config))
        }
        ProviderAdapterKind::Zai => runtime_adapter(zai::ZaiAdapter::new(config)),
        ProviderAdapterKind::MiniMax => runtime_adapter(minimax::MiniMaxAdapter::new(config)),
        ProviderAdapterKind::Mimo => runtime_adapter(mimo::MimoAdapter::new(config)),
    }
}

fn runtime_adapter(adapter: impl ProviderAdapter + 'static) -> Arc<dyn ProviderAdapter> {
    Arc::new(adapter)
}

fn api_endpoint(profile: ApiProfile) -> ApiEndpoint {
    match profile {
        ApiProfile::OpenAiResponses => ApiEndpoint::OpenAiResponses,
        ApiProfile::OpenAiChatCompletions => ApiEndpoint::OpenAiChatCompletions,
        ApiProfile::AnthropicMessages => ApiEndpoint::AnthropicMessages,
    }
}
