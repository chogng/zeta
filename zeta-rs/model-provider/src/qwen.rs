use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::Api;

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("qwen").expect("valid provider ID"),
        "Qwen",
        Api::Qwen,
        EndpointPolicy::ProviderDefault("https://dashscope.aliyuncs.com/compatible-mode/v1".into()),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_models([Model::new(
        ModelId::new("qwen-plus").expect("valid model ID"),
        "Qwen Plus",
    )])
    .expect("unique model IDs")
}
