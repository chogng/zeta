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
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct HuggingFaceAdapter {
    target: ResolvedApiTarget,
    endpoint: ApiEndpoint,
}

impl HuggingFaceAdapter {
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

impl ProviderAdapter for HuggingFaceAdapter {
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
}
