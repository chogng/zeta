use crate::{EndpointPolicy, ModelCatalogPolicy, Provider, ProviderAuthentication, ProviderId};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("huggingface").expect("valid provider ID"),
        "Hugging Face",
        Api::HuggingFace,
        EndpointPolicy::ProviderDefault("https://router.huggingface.co/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
}
