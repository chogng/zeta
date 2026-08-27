use super::ProviderAdapter;
use super::api_endpoint;
use super::stream_endpoint;
use crate::ModelProviderError;
use crate::provider::ModelEventSink;
use zeta_api::{ApiEndpoint, ApiProtocol, ModelRequest, ModelResponse};
use zeta_async_utils::CancellationToken;
use zeta_client::{OperationClient, ResolvedApiTarget};
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct OpenAiCompatibleAdapter {
    target: ResolvedApiTarget,
    endpoint: ApiEndpoint,
}

impl OpenAiCompatibleAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            target: ResolvedApiTarget::new(config.base_url.clone(), Vec::new()),
            endpoint: api_endpoint(config.api_profile),
        }
    }

    pub(crate) fn with_target(
        config: &NormalizedModelProviderConfig,
        target: ResolvedApiTarget,
    ) -> Self {
        Self {
            target,
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for OpenAiCompatibleAdapter {
    fn protocol(&self) -> ApiProtocol {
        self.endpoint.protocol()
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
