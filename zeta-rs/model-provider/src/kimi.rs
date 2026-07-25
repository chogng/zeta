use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("kimi").expect("valid provider ID"),
        "Kimi",
        Api::Kimi,
        EndpointPolicy::ProviderDefault("https://api.moonshot.ai/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_models([Model::new(
        ModelId::new("kimi-k2.6").expect("valid model ID"),
        "Kimi K2.6",
    )])
    .expect("unique model IDs")
}
