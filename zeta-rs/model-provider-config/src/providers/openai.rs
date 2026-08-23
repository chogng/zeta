use super::default_provider;
use crate::ApiProfile;
use crate::InputTokenCountDefinition;
use crate::InputTokenCountProfile;
use crate::ProviderAdapter;
use crate::ProviderDefinition;

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "openai",
        "OpenAI",
        ProviderAdapter::OpenAi,
        ApiProfile::OpenAiResponses,
        "https://api.openai.com/v1",
    )
    .with_input_token_count(InputTokenCountDefinition::invocation_base(
        InputTokenCountProfile::OpenAiResponses,
    ))
}
