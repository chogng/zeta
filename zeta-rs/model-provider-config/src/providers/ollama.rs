use super::default_provider;
use crate::{ApiProfile, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "ollama",
        "Ollama",
        ProviderAdapter::Ollama,
        ApiProfile::OpenAiChatCompletions,
        "http://localhost:11434/v1",
    )
}
