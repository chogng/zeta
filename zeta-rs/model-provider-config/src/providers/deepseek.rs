use super::default_provider;
use crate::{ApiProfile, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "deepseek",
        "DeepSeek",
        ProviderAdapter::DeepSeek,
        ApiProfile::OpenAiChatCompletions,
        "https://api.deepseek.com",
    )
}
