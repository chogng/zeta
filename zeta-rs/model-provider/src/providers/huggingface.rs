use super::ProviderAdapter;
use super::api_endpoint;
use zeta_api::ApiEndpoint;
use zeta_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct HuggingFaceAdapter {
    endpoint: ApiEndpoint,
}

impl HuggingFaceAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for HuggingFaceAdapter {
    fn endpoint(&self) -> ApiEndpoint {
        self.endpoint
    }
}
