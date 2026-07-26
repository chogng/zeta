use super::{ProviderAdapter, api_endpoint};
use crate::ModelProviderError;
use zeta_api::{ApiEndpoint, ApiProtocol, ModelRequest, ModelResponse};
use zeta_client::{OperationClient, ResolvedApiTarget};
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
    ) -> Result<ModelResponse, ModelProviderError> {
        self.endpoint
            .complete_with_client(&self.target, model, request, client)
            .map_err(Into::into)
    }
}
