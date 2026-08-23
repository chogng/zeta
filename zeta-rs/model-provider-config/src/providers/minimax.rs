use super::default_provider;
use crate::{ApiProfile, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "minimax",
        "MiniMax",
        ProviderAdapter::MiniMax,
        ApiProfile::OpenAiChatCompletions,
        "https://api.minimax.io/v1",
    )
}
