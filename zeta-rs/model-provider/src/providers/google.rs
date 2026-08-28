use super::ProviderAdapter;
use super::api_endpoint;
use super::stream_endpoint;
use crate::ModelProviderError;
use crate::provider::ModelEventSink;
use zeta_api::ApiEndpoint;
use zeta_api::ApiError;
use zeta_api::ApiProtocol;
use zeta_api::ModelRequest;
use zeta_api::ModelResponse;
use zeta_async_utils::CancellationToken;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_context_engine::ContextTokenMeasurementCapability;
use zeta_context_engine::ContextTokenMeasurementOutcome;
use zeta_http_client::HttpHeader;
use zeta_model_provider_config::InputTokenCountProfile;
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct GoogleAdapter {
    token_counter: Option<super::measurement::ProviderInputTokenCounter>,
    endpoint: ApiEndpoint,
}

impl GoogleAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            token_counter: super::measurement::ProviderInputTokenCounter::from_config(
                config,
                InputTokenCountProfile::GoogleGenerateContent,
            ),
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for GoogleAdapter {
    fn protocol(&self) -> ApiProtocol {
        self.endpoint.protocol()
    }

    fn fixed_headers(&self) -> Vec<HttpHeader> {
        vec![HttpHeader::new("x-goog-api-client", "zeta/0.1")]
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
        let count = match counter.count(target, model, request, client, cancellation) {
            Ok(count) => count,
            Err(ModelProviderError::Api(ApiError::InvalidRequest(_))) => {
                return Ok(ContextTokenMeasurementOutcome::Unavailable);
            }
            Err(error) => return Err(error),
        };
        super::measurement::estimated_provider_measurement(count, "google-gemini-count-tokens-v1")
    }

    fn complete(
        &self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, ModelProviderError> {
        self.endpoint
            .complete_with_client_and_cancellation(target, model, request, client, cancellation)
            .map_err(Into::into)
    }

    fn stream(
        &self,
        target: &ResolvedApiTarget,
        model: &str,
        request: &ModelRequest,
        client: &dyn OperationClient,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelEventSink,
    ) -> Result<ModelResponse, ModelProviderError> {
        stream_endpoint(
            self.endpoint,
            target,
            model,
            request,
            client,
            cancellation,
            sink,
        )
    }
}
