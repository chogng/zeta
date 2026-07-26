use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefaults, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "anthropic",
        "Anthropic",
        ProviderAdapter::Anthropic,
        ApiProfile::AnthropicMessages,
        "https://api.anthropic.com",
    )
    .with_models([Model::new(
        ModelId::new("claude-sonnet-4-20250514").expect("valid model ID"),
        "Claude Sonnet 4",
    )])
    .with_defaults(ProviderDefaults {
        max_output_tokens: Some(1024),
    })
}
