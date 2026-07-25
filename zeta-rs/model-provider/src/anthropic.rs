use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("anthropic").expect("valid provider ID"),
        "Anthropic",
        Api::Anthropic,
        EndpointPolicy::ProviderDefault("https://api.anthropic.com".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::ApiKeyHeader("x-api-key".into()),
    )
    .with_models([Model::new(
        ModelId::new("claude-sonnet-4-20250514").expect("valid model ID"),
        "Claude Sonnet 4",
    )])
    .expect("unique model IDs")
}
