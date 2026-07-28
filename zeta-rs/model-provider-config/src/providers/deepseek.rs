use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "deepseek",
        "DeepSeek",
        ProviderAdapter::DeepSeek,
        ApiProfile::OpenAiChatCompletions,
        "https://api.deepseek.com",
    )
    .with_default_model(Model::new(
        ModelId::new("deepseek-v4-pro").expect("valid model ID"),
        "DeepSeek V4 Pro",
    ))
}
