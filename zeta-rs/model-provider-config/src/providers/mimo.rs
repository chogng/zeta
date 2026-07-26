use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "mimo",
        "Xiaomi MiMo",
        ProviderAdapter::Mimo,
        ApiProfile::OpenAiChatCompletions,
        "https://api.xiaomimimo.com/v1",
    )
    .with_models([Model::new(
        ModelId::new("mimo-v2.5-pro").expect("valid model ID"),
        "MiMo V2.5 Pro",
    )])
}
