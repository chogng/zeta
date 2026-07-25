use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("mimo").expect("valid provider ID"),
        "Xiaomi MiMo",
        Api::Mimo,
        EndpointPolicy::ProviderDefault("https://api.xiaomimimo.com/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_models([Model::new(
        ModelId::new("mimo-v2.5-pro").expect("valid model ID"),
        "MiMo V2.5 Pro",
    )])
    .expect("unique model IDs")
}
