use super::ProviderAdapter;
use crate::ModelProviderError;
use zeta_api::ApiEndpoint;
use zeta_api::ApiProtocol;
use zeta_api::ModelRequest;
use zeta_api::ModelResponse;
use zeta_async_utils::CancellationToken;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct DeepSeekAdapter {
    endpoint: ApiEndpoint,
}

impl DeepSeekAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        let endpoint = match config.api_profile {
            zeta_model_provider_config::ApiProfile::OpenAiChatCompletions => {
                ApiEndpoint::DeepSeekChatCompletions
            }
            profile => super::api_endpoint(profile),
        };
        Self { endpoint }
    }
}

impl ProviderAdapter for DeepSeekAdapter {
    fn protocol(&self) -> ApiProtocol {
        self.endpoint.protocol()
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
}
