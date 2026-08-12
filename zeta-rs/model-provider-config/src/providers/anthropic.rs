use super::default_provider;
use crate::ApiProfile;
use crate::InputTokenCountDefinition;
use crate::InputTokenCountProfile;
use crate::Model;
use crate::ModelId;
use crate::ProviderAdapter;
use crate::ProviderDefaults;
use crate::ProviderDefinition;

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "anthropic",
        "Anthropic",
        ProviderAdapter::Anthropic,
        ApiProfile::AnthropicMessages,
        "https://api.anthropic.com",
    )
    .with_input_token_count(InputTokenCountDefinition::invocation_base(
        InputTokenCountProfile::AnthropicMessages,
    ))
    .with_defaults(ProviderDefaults {
        max_output_tokens: Some(1024),
        ..ProviderDefaults::default()
    })
    .with_default_model(Model::new(
        ModelId::new("claude-sonnet-4-20250514").expect("valid model ID"),
        "Claude Sonnet 4",
    ))
}
