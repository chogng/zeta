use super::configured_provider;
use crate::{ApiProfile, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    configured_provider(
        "openai-compatible",
        "OpenAI-compatible",
        ProviderAdapter::OpenAiCompatible,
        ApiProfile::OpenAiChatCompletions,
    )
    .with_native_streaming()
}
