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

pub(crate) struct AnthropicAdapter {
    target: ResolvedApiTarget,
    endpoint: ApiEndpoint,
}

impl AnthropicAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            target: ResolvedApiTarget::new(config.base_url.clone(), Vec::new()),
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for AnthropicAdapter {
    fn protocol(&self) -> ApiProtocol {
        self.endpoint.protocol()
    }

    fn input_token_measurement_capability(&self) -> ContextTokenMeasurementCapability {
        match self.endpoint {
            ApiEndpoint::AnthropicMessages => ContextTokenMeasurementCapability::Remote,
            ApiEndpoint::OpenAiResponses | ApiEndpoint::OpenAiChatCompletions => {
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
        if self.endpoint != ApiEndpoint::AnthropicMessages {
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
        let expected = ContextTokenCount::new(count);
        let uncertainty = ContextTokenCount::new(count.div_ceil(100).max(32));
        let conservative_input = expected.saturating_add(uncertainty);
        let source = ContextTokenMeasurementSource::provider_preflight("anthropic-count-tokens-v1")
            .expect("measurement source revision is constant and non-empty");
        let measurement = ContextTokenMeasurement::estimated(expected, conservative_input, source)
            .expect("the Anthropic uncertainty policy cannot lower the reported count");
        Ok(ContextTokenMeasurementOutcome::Measured(measurement))
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
