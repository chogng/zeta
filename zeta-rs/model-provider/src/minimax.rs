use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("minimax").expect("valid provider ID"),
        "MiniMax",
        Api::MiniMax,
        EndpointPolicy::ProviderDefault("https://api.minimax.io/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_models([Model::new(
        ModelId::new("MiniMax-M3").expect("valid model ID"),
        "MiniMax M3",
    )])
    .expect("unique model IDs")
}
