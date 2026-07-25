use crate::{
    EndpointPolicy, Model, ModelCatalogPolicy, ModelId, Provider, ProviderAuthentication,
    ProviderId,
};
use zeta_api::{Api, HttpHeader};

pub(crate) fn provider() -> Provider {
    Provider::new(
        ProviderId::new("google").expect("valid provider ID"),
        "Google (Gemini)",
        Api::Google,
        EndpointPolicy::ProviderDefault(
            "https://generativelanguage.googleapis.com/v1beta/openai".into(),
        ),
        ModelCatalogPolicy::AllowUnlisted,
        ProviderAuthentication::Bearer,
    )
    .with_headers([HttpHeader::new("x-goog-api-client", "zeta/0.1")])
    .with_models([Model::new(
        ModelId::new("gemini-3.6-flash").expect("valid model ID"),
        "Gemini 3.6 Flash",
    )])
    .expect("unique model IDs")
}
