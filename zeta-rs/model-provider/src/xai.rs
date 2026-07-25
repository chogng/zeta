use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("xai").expect("valid provider ID"),
        "xAI (Grok)",
        Api::Xai,
        EndpointPolicy::ProviderDefault("https://api.x.ai/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_models([Model::new(
        ModelId::new("grok-4.5").expect("valid model ID"),
        "Grok 4.5",
    )])
    .expect("unique model IDs")
}
