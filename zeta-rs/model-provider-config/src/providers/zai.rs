use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "zai",
        "Z.AI (GLM)",
        ProviderAdapter::Zai,
        ApiProfile::OpenAiChatCompletions,
        "https://api.z.ai/api/paas/v4",
    )
    .with_default_model(Model::new(
        ModelId::new("glm-5.1").expect("valid model ID"),
        "GLM-5.1",
    ))
}
