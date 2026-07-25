use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::{Api, HttpHeader};

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("zai").expect("valid provider ID"),
        "Z.AI (GLM)",
        Api::Zai,
        EndpointPolicy::ProviderDefault("https://api.z.ai/api/paas/v4".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_headers([HttpHeader::new("Accept-Language", "en-US,en")])
    .with_models([Model::new(
        ModelId::new("glm-5.1").expect("valid model ID"),
        "GLM-5.1",
    )])
    .expect("unique model IDs")
}
