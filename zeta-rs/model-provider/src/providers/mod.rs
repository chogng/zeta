use crate::ModelProviderError;
use std::sync::Arc;
use zeta_api::{ApiEndpoint, ApiProtocol, ModelRequest, ModelResponse};
use zeta_async_utils::CancellationToken;
use zeta_client::OperationClient;
use zeta_model_provider_config::{
    ApiProfile, NormalizedModelProviderConfig, ProviderAdapter as ProviderAdapterKind,
};

mod anthropic;
mod deepseek;
mod google;
mod huggingface;
mod kimi;
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

    fn complete(
        &self,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError>;
}

pub(crate) fn instantiate(
    adapter: ProviderAdapterKind,
    config: &NormalizedModelProviderConfig,
) -> Arc<dyn ProviderAdapter> {
    match adapter {
        ProviderAdapterKind::OpenAi => runtime_adapter(openai::OpenAiAdapter::new(config)),
        ProviderAdapterKind::OpenAiCompatible => {
            runtime_adapter(openai_compatible::OpenAiCompatibleAdapter::new(config))
        }
        ProviderAdapterKind::Anthropic => runtime_adapter(anthropic::AnthropicAdapter::new(config)),
        ProviderAdapterKind::Google => runtime_adapter(google::GoogleAdapter::new(config)),
        ProviderAdapterKind::Xai => runtime_adapter(xai::XaiAdapter::new(config)),
        ProviderAdapterKind::Qwen => runtime_adapter(qwen::QwenAdapter::new(config)),
        ProviderAdapterKind::Kimi => runtime_adapter(kimi::KimiAdapter::new(config)),
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
