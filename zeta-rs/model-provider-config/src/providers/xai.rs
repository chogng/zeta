use super::default_provider;
use crate::{ApiProfile, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "xai",
        "xAI (Grok)",
        ProviderAdapter::Xai,
        ApiProfile::OpenAiChatCompletions,
        "https://api.x.ai/v1",
    )
}
