use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "kimi",
        "Kimi",
        ProviderAdapter::Kimi,
        ApiProfile::OpenAiChatCompletions,
        "https://api.moonshot.ai/v1",
    )
    .with_default_model(Model::new(
        ModelId::new("kimi-k2.6").expect("valid model ID"),
        "Kimi K2.6",
    ))
}
