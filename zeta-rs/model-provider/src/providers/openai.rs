use super::ProviderAdapter;
use super::api_endpoint;
use super::stream_endpoint;
use crate::ModelProviderError;
use crate::provider::ModelEventSink;
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
use zeta_model_provider_config::InputTokenCountProfile;
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct OpenAiAdapter {
    target: ResolvedApiTarget,
    token_counter: Option<super::measurement::ProviderInputTokenCounter>,
    endpoint: ApiEndpoint,
}

impl OpenAiAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            target: ResolvedApiTarget::new(config.base_url.clone(), Vec::new()),
            token_counter: super::measurement::ProviderInputTokenCounter::from_config(
                config,
                Vec::new(),
                InputTokenCountProfile::OpenAiResponses,
            ),
            endpoint: api_endpoint(config.api_profile),
        }
    }

    pub(crate) fn with_target(
        config: &NormalizedModelProviderConfig,
        target: ResolvedApiTarget,
        supports_input_measurement: bool,
    ) -> Self {
        let token_counter = supports_input_measurement
            .then(|| {
                super::measurement::ProviderInputTokenCounter::from_config(
                    config,
                    target.headers.clone(),
                    InputTokenCountProfile::OpenAiResponses,
                )
            })
            .flatten();
        Self {
            target,
            token_counter,
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for OpenAiAdapter {
    fn protocol(&self) -> ApiProtocol {
        self.endpoint.protocol()
    }

    fn input_token_measurement_capability(&self, model: &str) -> ContextTokenMeasurementCapability {
        if self
            .token_counter
            .as_ref()
            .is_some_and(|counter| counter.supports(model))
        {
            ContextTokenMeasurementCapability::Remote
        } else {
            ContextTokenMeasurementCapability::Unavailable
        }
    }

    fn measure_input(
        &self,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, ModelProviderError> {
        let Some(counter) = self
            .token_counter
            .as_ref()
            .filter(|counter| counter.supports(model))
        else {
            return Ok(ContextTokenMeasurementOutcome::Unavailable);
        };
        let count = counter.count(model, request, client, cancellation)?;
        let count = u32::try_from(count.get()).map_err(|_| {
            ModelProviderError::InvalidResponse("input token count exceeds supported range".into())
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

    fn stream(
        &self,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelProviderError> {
        stream_endpoint(
            self.endpoint,
            &self.target,
            model,
            request,
            client,
            cancellation,
            sink,
        )
    }
}
