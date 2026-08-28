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
use zeta_context_engine::ContextTokenMeasurementCapability;
use zeta_context_engine::ContextTokenMeasurementOutcome;
use zeta_model_provider_config::InputTokenCountProfile;
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct KimiAdapter {
    token_counter: Option<super::measurement::ProviderInputTokenCounter>,
    endpoint: ApiEndpoint,
}

impl KimiAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            token_counter: super::measurement::ProviderInputTokenCounter::from_config(
                config,
                InputTokenCountProfile::KimiChatCompletions,
            ),
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for KimiAdapter {
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
        target: &ResolvedApiTarget,
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
        if !request.tools.is_empty() || request.reasoning.is_some() {
            return Ok(ContextTokenMeasurementOutcome::Unavailable);
        }
        let count = counter.count(target, model, request, client, cancellation)?;
        super::measurement::estimated_provider_measurement(count, "kimi-estimate-token-count-v1")
    }

    fn complete(
        &self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError> {
        let model = upstream_model(model);
        self.endpoint
            .complete_with_client_and_cancellation(target, model, request, client, cancellation)
            .map_err(Into::into)
    }
}

fn upstream_model(model: &str) -> &str {
    match model {
        "kimi-k2.7-code" => "kimi-for-coding",
        "kimi-k2.7-code-highspeed" => "kimi-for-coding-highspeed",
        _ => model,
    }
}
