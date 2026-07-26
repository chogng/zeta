use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "minimax",
        "MiniMax",
        ProviderAdapter::MiniMax,
        ApiProfile::OpenAiChatCompletions,
        "https://api.minimax.io/v1",
    )
    .with_models([Model::new(
        ModelId::new("MiniMax-M3").expect("valid model ID"),
        "MiniMax M3",
    )])
}
