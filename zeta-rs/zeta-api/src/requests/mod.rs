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
    let body = serde_json::to_vec(&body)
        .map_err(|error| ApiError::InvalidRequest(format!("failed to encode API JSON: {error}")))?;
    let request = ClientRequest::new(
        endpoint.method(),
        target.endpoint(endpoint.relative_path())?,
        endpoint.headers(target),
        body,
        target.retry_policy,
    )?;
    let response = client.execute_with_cancellation(&request, cancellation)?;
    if !response.is_success() {
        return Err(match response.status() {
            429 => ApiError::RateLimited {
                retry_after_ms: response
                    .retry_after()
                    .and_then(|delay| u64::try_from(delay.as_millis()).ok())
                    .map(|delay| delay.min(60_000)),
            },
            500..=599 => ApiError::Overloaded,
            status => ApiError::HttpStatus(status),
        });
    }
    serde_json::from_slice(response.body())
        .map_err(|_| ApiError::InvalidResponse("server returned invalid JSON".into()))
}

pub(crate) mod anthropic_messages;
pub(crate) mod openai_chat_completions;
pub(crate) mod openai_responses;
