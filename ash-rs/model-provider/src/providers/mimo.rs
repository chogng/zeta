use super::ProviderAdapter;
use super::api_endpoint;
use ash_api::ApiEndpoint;
use ash_model_provider_config::NormalizedModelProviderConfig;

pub(crate) struct MimoAdapter {
    endpoint: ApiEndpoint,
}

impl MimoAdapter {
    pub(crate) fn new(config: &NormalizedModelProviderConfig) -> Self {
        Self {
            endpoint: api_endpoint(config.api_profile),
        }
    }
}

impl ProviderAdapter for MimoAdapter {
    fn endpoint(&self) -> ApiEndpoint {
        self.endpoint
    }
}
