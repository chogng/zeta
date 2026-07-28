use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "openai",
        "OpenAI",
        ProviderAdapter::OpenAi,
        ApiProfile::OpenAiResponses,
        "https://api.openai.com/v1",
    )
    .with_default_model(Model::new(
        ModelId::new("gpt-5.6").expect("valid model ID"),
        "GPT-5.6",
    ))
}
