use super::ProviderAdapter;
use super::api_endpoint;
use crate::ModelProviderError;
use zeta_api::ApiEndpoint;
use zeta_api::ApiProtocol;
use zeta_api::ModelRequest;
use zeta_api::ModelResponse;
use zeta_async_utils::CancellationToken;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_context_engine::ContextTokenCount;
use zeta_context_engine::ContextTokenMeasurement;
use zeta_context_engine::ContextTokenMeasurementCapability;
use zeta_context_engine::ContextTokenMeasurementOutcome;
use zeta_context_engine::ContextTokenMeasurementSource;
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct OpenAiAdapter {
    target: ResolvedApiTarget,
    endpoint: ApiEndpoint,
}

impl OpenAiAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            target: ResolvedApiTarget::new(config.base_url.clone(), Vec::new()),
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for OpenAiAdapter {
    fn protocol(&self) -> ApiProtocol {
        self.endpoint.protocol()
    }

    fn input_token_measurement_capability(&self) -> ContextTokenMeasurementCapability {
        match self.endpoint {
            ApiEndpoint::OpenAiResponses => ContextTokenMeasurementCapability::Remote,
            ApiEndpoint::OpenAiChatCompletions | ApiEndpoint::AnthropicMessages => {
                ContextTokenMeasurementCapability::Unavailable
            }
        }
    }

    fn measure_input(
        &self,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, ModelProviderError> {
        if self.endpoint != ApiEndpoint::OpenAiResponses {
            return Ok(ContextTokenMeasurementOutcome::Unavailable);
        }
        let count = self
            .endpoint
            .count_input_tokens_with_client_and_cancellation(
                &self.target,
                model,
                request,
                client,
                cancellation,
            )?;
        let count = u32::try_from(count.get()).map_err(|_| {
            ModelProviderError::InvalidResponse("input token count exceeds supported range")
        })?;
        let source =
            ContextTokenMeasurementSource::provider_preflight("openai-responses-input-tokens-v1")
                .expect("measurement source revision is constant and non-empty");
        Ok(ContextTokenMeasurementOutcome::Measured(
            ContextTokenMeasurement::exact(ContextTokenCount::new(count), source),
        ))
    }

    fn complete(
        &self,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError> {
        self.endpoint
            .complete_with_client_and_cancellation(
                &self.target,
                model,
                request,
                client,
                cancellation,
            )
            .map_err(Into::into)
    }
}
