use crate::{EndpointPolicy, ModelCatalogPolicy, Provider, ProviderAuthentication, ProviderId};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("ollama").expect("valid provider ID"),
        "Ollama",
        Api::Ollama,
        EndpointPolicy::ProviderDefault("http://localhost:11434/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::None,
    )
}
