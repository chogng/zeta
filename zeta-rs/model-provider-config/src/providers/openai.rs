use super::default_provider;
use crate::ApiProfile;
use crate::InputTokenCountDefinition;
use crate::InputTokenCountProfile;
use crate::Model;
use crate::ModelId;
use crate::ProviderAdapter;
use crate::ProviderDefinition;
use zeta_protocol::CapabilitySupport;

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
    .with_default_model({
        let mut model = Model::new(ModelId::new("gpt-5.6").expect("valid model ID"), "GPT-5.6");
        model.capabilities.image_detail_original = CapabilitySupport::Supported;
        model
    })
}
