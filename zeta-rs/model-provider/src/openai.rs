use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("openai").expect("valid provider ID"),
        "OpenAI",
        Api::OpenAi,
        EndpointPolicy::ProviderDefault("https://api.openai.com/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_models([Model::new(
        ModelId::new("gpt-5.6").expect("valid model ID"),
        "GPT-5.6",
    )])
    .expect("unique model IDs")
}
