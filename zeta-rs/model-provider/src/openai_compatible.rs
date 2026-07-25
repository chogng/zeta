use crate::{EndpointPolicy, ModelCatalogPolicy, Provider, ProviderAuthentication, ProviderId};
use zeta_api::Api;
use zeta_core::{AgentModel, CoreError};

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("openai-compatible").expect("valid provider ID"),
        "OpenAI-compatible",
        Api::OpenAiCompatible,
        EndpointPolicy::ConfiguredOnly,
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
}

/// Returns a clear model error until the local host has selected a provider configuration.
pub struct UnavailableModel {
    message: String,
}

impl UnavailableModel {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl AgentModel for UnavailableModel {
    fn respond(&self, _: &str) -> Result<String, CoreError> {
        Err(CoreError::Model(self.message.clone()))
    }
}

/// Deterministic model adapter used only by unit tests and local protocol fixtures.
pub struct EchoModel;

impl AgentModel for EchoModel {
    fn respond(&self, prompt: &str) -> Result<String, CoreError> {
        Ok(format!("Zeta: {prompt}"))
    }
}
