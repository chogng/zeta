//! Endpoint-family dispatch for normalized model requests.
//!
//! This module deliberately names wire-protocol endpoint families, not model
//! vendors. Provider selection, credentials, base URLs, and vendor-specific
//! headers belong to `zeta-model-provider`; this crate only selects a codec
//! after the runtime has resolved those concerns.

use crate::ApiError;
use crate::InputTokenCount;
use crate::ModelRequest;
use crate::ModelResponse;
use crate::requests;
use zeta_async_utils::CancellationSource;
use zeta_async_utils::CancellationToken;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_http_client::HttpHeader;

const ANTHROPIC_MESSAGES_API_VERSION: &str = "2023-06-01";

/// The normalized protocol spoken by an API endpoint family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiProtocol {
    /// OpenAI's Responses API request and response schema.
    OpenAiResponses,
    /// The OpenAI Chat Completions-compatible request and response schema.
    OpenAiCompletions,
    /// Anthropic's Messages API request and response schema.
    AnthropicMessages,
}

/// A provider-independent API endpoint family supported by Zeta.
///
/// A caller supplies a resolved base URL and headers. This type then encodes a
/// normalized request and decodes the corresponding provider response without
/// learning which provider supplied the endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiEndpoint {
    /// An endpoint implementing the OpenAI Responses API.
    OpenAiResponses,
    /// An endpoint implementing the OpenAI Chat Completions-compatible API.
    OpenAiChatCompletions,
    /// An endpoint implementing Anthropic's Messages API.
    AnthropicMessages,
}

impl ApiEndpoint {
    /// Returns the wire protocol implemented by this endpoint family.
    pub fn protocol(self) -> ApiProtocol {
        match self {
            Self::OpenAiResponses => ApiProtocol::OpenAiResponses,
            Self::OpenAiChatCompletions => ApiProtocol::OpenAiCompletions,
            Self::AnthropicMessages => ApiProtocol::AnthropicMessages,
        }
    }

    pub(crate) fn relative_path(self) -> &'static str {
        match self {
            Self::OpenAiResponses => "responses",
            Self::OpenAiChatCompletions => "chat/completions",
            Self::AnthropicMessages => "v1/messages",
        }
    }

    pub(crate) fn headers(self, target: &ResolvedApiTarget) -> Vec<HttpHeader> {
        let mut headers = target.headers.clone();
        if self == Self::AnthropicMessages
            && !headers
                .iter()
                .any(|header| header.name().eq_ignore_ascii_case("anthropic-version"))
        {
            headers.push(HttpHeader::new(
                "anthropic-version",
                ANTHROPIC_MESSAGES_API_VERSION,
            ));
        }
        headers
    }

    /// Completes a normalized request through a supplied operation client.
    ///
    /// The transport only owns HTTP execution. This method owns request and
    /// response codec selection for the endpoint family.
    pub fn complete_with_client(
        self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
    ) -> Result<ModelResponse, ApiError> {
        self.complete_with_client_and_cancellation(
            target,
            model,
            request,
            client,
            &CancellationSource::new().token(),
        )
    }

    /// Completes a normalized request while observing one caller-owned cancellation scope.
    pub fn complete_with_client_and_cancellation(
        self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ApiError> {
        validate_request(model, request)?;
        match self {
            Self::OpenAiResponses => requests::openai_responses::complete(
                self,
                target,
                model,
                request,
                client,
                cancellation,
            ),
            Self::OpenAiChatCompletions => requests::openai_chat_completions::complete(
                self,
                target,
                model,
                request,
                client,
                cancellation,
            ),
            Self::AnthropicMessages => requests::anthropic_messages::complete(
                self,
                target,
                model,
                request,
                client,
                cancellation,
            ),
        }
    }

    /// Counts the input tokens for one normalized request through a supported provider preflight
    /// endpoint.
    pub fn count_input_tokens_with_client(
        self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
    ) -> Result<InputTokenCount, ApiError> {
        self.count_input_tokens_with_client_and_cancellation(
            target,
            model,
            request,
            client,
            &CancellationSource::new().token(),
        )
    }

    /// Counts input tokens while observing one caller-owned cancellation scope.
    pub fn count_input_tokens_with_client_and_cancellation(
        self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<InputTokenCount, ApiError> {
        validate_request(model, request)?;
        match self {
            Self::OpenAiResponses => requests::openai_responses::count_input_tokens(
                self,
                target,
                model,
                request,
                client,
                cancellation,
            ),
            Self::AnthropicMessages => requests::anthropic_messages::count_input_tokens(
                self,
                target,
                model,
                request,
                client,
                cancellation,
            ),
            Self::OpenAiChatCompletions => Err(ApiError::InvalidRequest(
                "OpenAI Chat Completions does not expose a standard input-token count endpoint"
                    .into(),
            )),
        }
    }
}

pub(crate) fn validate_request(model: &str, request: &ModelRequest) -> Result<(), ApiError> {
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
