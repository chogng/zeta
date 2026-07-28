use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "google",
        "Google (Gemini)",
        ProviderAdapter::Google,
        ApiProfile::OpenAiChatCompletions,
        "https://generativelanguage.googleapis.com/v1beta/openai",
    )
    .with_default_model(Model::new(
        ModelId::new("gemini-3.6-flash").expect("valid model ID"),
        "Gemini 3.6 Flash",
    ))
}
