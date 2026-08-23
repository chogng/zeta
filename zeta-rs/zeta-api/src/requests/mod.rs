//! Request and unary-response codecs grouped by API endpoint family.
//!
//! Each codec receives its method, relative path, and protocol-owned headers
//! from `endpoint/`; it only owns JSON/body conversion.

use crate::{ApiEndpoint, ApiError};
use serde_json::Value;
use zeta_async_utils::CancellationToken;
use zeta_client::{ClientRequest, OperationClient, ResolvedApiTarget};

pub(crate) fn post_json(
    client: &dyn OperationClient,
    target: &ResolvedApiTarget,
    endpoint: ApiEndpoint,
    body: Value,
    cancellation: &CancellationToken,
) -> Result<Value, ApiError> {
    post_json_to_path(
        client,
        target,
        endpoint.relative_path(),
        endpoint.headers(target),
        body,
        cancellation,
    )
}

pub(crate) fn post_json_to_path(
    client: &dyn OperationClient,
    target: &ResolvedApiTarget,
    relative_path: &str,
    headers: Vec<zeta_http_client::HttpHeader>,
    body: Value,
    cancellation: &CancellationToken,
) -> Result<Value, ApiError> {
    let body = serde_json::to_vec(&body)
        .map_err(|error| ApiError::InvalidRequest(format!("failed to encode API JSON: {error}")))?;
    let request = ClientRequest::new(
        zeta_http_client::HttpMethod::Post,
        target.endpoint(relative_path)?,
        headers,
        body,
        target.retry_policy,
    )?;
    let response = client.execute_with_cancellation(&request, cancellation)?;
    if !response.is_success() {
        return Err(response_error(&response));
    }
    serde_json::from_slice(response.body())
        .map_err(|_| ApiError::InvalidResponse("server returned invalid JSON".into()))
}

pub(crate) fn response_error(response: &zeta_client::ClientResponse) -> ApiError {
    match response.status() {
        429 => ApiError::RateLimited {
            retry_after_ms: response
                .retry_after()
                .and_then(|delay| u64::try_from(delay.as_millis()).ok())
                .map(|delay| delay.min(60_000)),
        },
        500..=599 => ApiError::Overloaded,
        401 | 403 => ApiError::AuthFailed(provider_error_detail(response.body())),
        400 => classify_provider_error(response.body(), ProviderErrorFallback::InvalidRequest),
        status => classify_provider_error(response.body(), ProviderErrorFallback::Status(status)),
    }
}

pub(crate) fn stream_error(body: &str) -> ApiError {
    classify_provider_error(body.as_bytes(), ProviderErrorFallback::InvalidResponse)
}

#[derive(Clone, Copy)]
enum ProviderErrorFallback {
    InvalidRequest,
    InvalidResponse,
    Status(u16),
}

fn classify_provider_error(body: &[u8], fallback: ProviderErrorFallback) -> ApiError {
    let detail = provider_error_detail(body);
    let normalized = detail.to_ascii_lowercase();
    if is_context_overflow(&normalized) {
        return ApiError::ContextOverflow(detail);
    }
    if is_auth_failure(&normalized) {
        return ApiError::AuthFailed(detail);
    }
    if is_overloaded(&normalized) {
        return ApiError::Overloaded;
    }
    if is_invalid_request(&normalized) {
        return ApiError::InvalidRequest(detail);
    }
    match fallback {
        ProviderErrorFallback::InvalidRequest => ApiError::InvalidRequest(detail),
        ProviderErrorFallback::InvalidResponse => ApiError::InvalidResponse(detail),
        ProviderErrorFallback::Status(status) => ApiError::HttpStatus(status),
    }
}

fn provider_error_detail(body: &[u8]) -> String {
    const MAX_PROVIDER_ERROR_BYTES: usize = 4 * 1024;
    let length = body.len().min(MAX_PROVIDER_ERROR_BYTES);
    let mut detail = String::from_utf8_lossy(&body[..length]).into_owned();
    if detail.trim().is_empty() {
        return "provider returned an empty error body".into();
    }
    if body.len() > length {
        detail.push_str(" [truncated]");
    }
    detail
}

fn is_context_overflow(detail: &str) -> bool {
    detail.contains("context_length_exceeded")
        || detail.contains("context window")
        || detail.contains("maximum context length")
        || detail.contains("prompt is too long")
        || detail.contains("too many tokens")
        || (detail.contains("input token")
            && detail.contains("maximum")
            && (detail.contains("exceed") || detail.contains("limit")))
        || (detail.contains("context")
            && (detail.contains("length") || detail.contains("token"))
            && (detail.contains("exceed")
                || detail.contains("limit")
                || detail.contains("maximum")
                || detail.contains("too long")))
}

fn is_auth_failure(detail: &str) -> bool {
    detail.contains("authentication_error")
        || detail.contains("invalid_api_key")
        || detail.contains("invalid api key")
        || detail.contains("incorrect api key")
        || detail.contains("permission_denied")
}

fn is_overloaded(detail: &str) -> bool {
    detail.contains("overloaded_error") || detail.contains("server_overloaded")
}

fn is_invalid_request(detail: &str) -> bool {
    detail.contains("invalid_request_error") || detail.contains("invalid_argument")
}

pub(crate) mod anthropic_messages;
pub(crate) mod google_count_tokens;
pub(crate) mod kimi_estimate_tokens;
pub(crate) mod openai_chat_completions;
pub(crate) mod openai_responses;
pub(crate) mod zai_tokenizer;
