use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("deepseek").expect("valid provider ID"),
        "DeepSeek",
        Api::DeepSeek,
        EndpointPolicy::ProviderDefault("https://api.deepseek.com".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_models([Model::new(
        ModelId::new("deepseek-v4-pro").expect("valid model ID"),
        "DeepSeek V4 Pro",
    )])
    .expect("unique model IDs")
}
