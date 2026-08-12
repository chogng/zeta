use super::default_provider;
use crate::ApiProfile;
use crate::InputTokenCountDefinition;
use crate::InputTokenCountProfile;
use crate::Model;
use crate::ModelId;
use crate::ProviderAdapter;
use crate::ProviderDefinition;

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "google",
        "Google (Gemini)",
        ProviderAdapter::Google,
        ApiProfile::OpenAiChatCompletions,
        "https://generativelanguage.googleapis.com/v1beta/openai",
    )
    .with_input_token_count(
        InputTokenCountDefinition::provider_default(
            InputTokenCountProfile::GoogleGenerateContent,
            "https://generativelanguage.googleapis.com/v1beta",
        )
        .with_models([ModelId::new("gemini-3.6-flash").expect("valid model ID")]),
    )
    .with_default_model(Model::new(
        ModelId::new("gemini-3.6-flash").expect("valid model ID"),
        "Gemini 3.6 Flash",
    ))
}
