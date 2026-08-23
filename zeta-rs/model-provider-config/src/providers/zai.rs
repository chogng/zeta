use super::default_provider;
use crate::ApiProfile;
use crate::InputTokenCountDefinition;
use crate::InputTokenCountProfile;
use crate::ModelId;
use crate::ProviderAdapter;
use crate::ProviderDefinition;

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "zai",
        "Z.AI (GLM)",
        ProviderAdapter::Zai,
        ApiProfile::OpenAiChatCompletions,
        "https://api.z.ai/api/paas/v4",
    )
    .with_input_token_count(
        InputTokenCountDefinition::invocation_base(InputTokenCountProfile::ZaiChatCompletions)
            .with_models([
                ModelId::new("glm-4.6").expect("valid model ID"),
                ModelId::new("glm-4.6v").expect("valid model ID"),
                ModelId::new("glm-4.5").expect("valid model ID"),
            ]),
    )
}
