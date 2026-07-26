use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "xai",
        "xAI (Grok)",
        ProviderAdapter::Xai,
        ApiProfile::OpenAiChatCompletions,
        "https://api.x.ai/v1",
    )
    .with_models([Model::new(
        ModelId::new("grok-4.5").expect("valid model ID"),
        "Grok 4.5",
    )])
}
