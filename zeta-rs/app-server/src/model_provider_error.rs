use zeta_core::CoreError;
use zeta_model_provider::ApiError;
use zeta_model_provider::ModelProviderError;

pub(super) fn map_model_provider_error(error: ModelProviderError) -> CoreError {
    if let ModelProviderError::Cancelled(message) = &error {
        return CoreError::Cancelled(message.clone());
    }

    let retry_after_ms = error
        .retry_after()
        .and_then(|delay| u64::try_from(delay.as_millis()).ok());
    let mapped = match &error {
        ModelProviderError::ContextOverflow(_)
        | ModelProviderError::Api(ApiError::ContextOverflow(_)) => CoreError::ModelContextOverflow,
        ModelProviderError::AuthFailed(_)
        | ModelProviderError::Credential(_)
        | ModelProviderError::Api(ApiError::AuthFailed(_)) => CoreError::ModelAuthFailed,
        ModelProviderError::InvalidRequest(_)
        | ModelProviderError::Api(ApiError::InvalidRequest(_)) => CoreError::ModelInvalidRequest,
        ModelProviderError::InvalidResponse(_)
        | ModelProviderError::Api(ApiError::InvalidResponse(_)) => CoreError::ModelInvalidResponse,
        _ if error.is_transient() => CoreError::ModelTransient { retry_after_ms },
        _ => CoreError::Model("model invocation failed".into()),
    };
    log::debug!("model provider invocation failed: {error:?}");
    mapped
}

#[cfg(test)]
#[path = "model_provider_error_tests.rs"]
mod tests;
