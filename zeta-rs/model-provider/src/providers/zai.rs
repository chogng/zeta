use super::{ProviderAdapter, api_endpoint};
use crate::ModelProviderError;
use zeta_api::{ApiEndpoint, ApiProtocol, ModelRequest, ModelResponse};
use zeta_async_utils::CancellationToken;
use zeta_client::{OperationClient, ResolvedApiTarget};
use zeta_http_client::HttpHeader;
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct ZaiAdapter {
    target: ResolvedApiTarget,
    endpoint: ApiEndpoint,
}

impl ZaiAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            target: ResolvedApiTarget::new(
                config.base_url.clone(),
                vec![HttpHeader::new("Accept-Language", "en-US,en")],
            ),
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for ZaiAdapter {
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
