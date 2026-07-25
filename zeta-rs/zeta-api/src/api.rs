use crate::{
    ApiError, JsonHttpTransport, ModelRequest, ModelResponse, ResolvedApiTarget,
    UreqJsonHttpTransport, anthropic, deepseek, google, huggingface, kimi, mimo, minimax, ollama,
    openai, openai_compatible, qwen, xai, zai,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiProtocol {
    OpenAiResponses,
    OpenAiCompletions,
    AnthropicMessages,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Api {
    OpenAi,
    OpenAiCompatible,
    Anthropic,
    Google,
    Xai,
    Qwen,
    Kimi,
    DeepSeek,
    Ollama,
    HuggingFace,
    Zai,
    MiniMax,
    Mimo,
}

impl Api {
    pub fn protocol(&self) -> ApiProtocol {
        match self {
            Self::OpenAi => ApiProtocol::OpenAiResponses,
            Self::Anthropic => ApiProtocol::AnthropicMessages,
            Self::OpenAiCompatible
            | Self::Google
            | Self::Xai
            | Self::Qwen
            | Self::Kimi
            | Self::DeepSeek
            | Self::Ollama
            | Self::HuggingFace
            | Self::Zai
            | Self::MiniMax
            | Self::Mimo => ApiProtocol::OpenAiCompletions,
        }
    }

    pub fn complete(
        &self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ApiError> {
        self.complete_with_transport(target, model, request, &UreqJsonHttpTransport::new())
    }

    pub fn complete_with_transport(
        &self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        transport: &dyn JsonHttpTransport,
    ) -> Result<ModelResponse, ApiError> {
        validate_request(model, request)?;
        match self {
            Self::OpenAi => openai::complete(target, model, request, transport),
            Self::OpenAiCompatible => {
                openai_compatible::complete(target, model, request, transport)
            }
            Self::Anthropic => anthropic::complete(target, model, request, transport),
            Self::Google => google::complete(target, model, request, transport),
            Self::Xai => xai::complete(target, model, request, transport),
            Self::Qwen => qwen::complete(target, model, request, transport),
            Self::Kimi => kimi::complete(target, model, request, transport),
            Self::DeepSeek => deepseek::complete(target, model, request, transport),
            Self::Ollama => ollama::complete(target, model, request, transport),
            Self::HuggingFace => huggingface::complete(target, model, request, transport),
            Self::Zai => zai::complete(target, model, request, transport),
            Self::MiniMax => minimax::complete(target, model, request, transport),
            Self::Mimo => mimo::complete(target, model, request, transport),
        }
    }
}

fn validate_request(model: &str, request: &ModelRequest) -> Result<(), ApiError> {
    if model.trim().is_empty() {
        return Err(ApiError::InvalidRequest("model must not be empty".into()));
    }
    if request.input.is_empty() {
        return Err(ApiError::InvalidRequest("input must not be empty".into()));
    }
    if request.max_output_tokens == Some(0) {
        return Err(ApiError::InvalidRequest(
            "maximum output tokens must be greater than zero".into(),
        ));
    }
    Ok(())
}
