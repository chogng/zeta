use super::ProviderAdapter;
use super::api_endpoint;
use crate::ModelProviderError;
use zeta_api::ApiEndpoint;
use zeta_api::ApiProtocol;
use zeta_api::InputItem;
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

pub(crate) struct ZaiAdapter {
    target: ResolvedApiTarget,
    token_counter: Option<super::measurement::ProviderInputTokenCounter>,
    endpoint: ApiEndpoint,
}

impl ZaiAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        let headers = vec![HttpHeader::new("Accept-Language", "en-US,en")];
        Self {
            target: ResolvedApiTarget::new(config.base_url.clone(), headers.clone()),
            token_counter: super::measurement::ProviderInputTokenCounter::from_config(
                config,
                headers,
                InputTokenCountProfile::ZaiChatCompletions,
            ),
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for ZaiAdapter {
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
        if contains_tool_history(request) {
            return Ok(ContextTokenMeasurementOutcome::Unavailable);
        }
        let count = counter.count(model, request, client, cancellation)?;
        super::measurement::estimated_provider_measurement(count, "zai-tokenizer-v1")
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

fn contains_tool_history(request: &ModelRequest) -> bool {
    request.input.iter().any(|item| match item {
        InputItem::Message(message) => !message.tool_calls.is_empty(),
        InputItem::ToolResult(_) => true,
    })
}
