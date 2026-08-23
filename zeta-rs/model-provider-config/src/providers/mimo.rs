use super::default_provider;
use crate::{ApiProfile, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "mimo",
        "Xiaomi MiMo",
        ProviderAdapter::Mimo,
        ApiProfile::OpenAiChatCompletions,
        "https://api.xiaomimimo.com/v1",
    )
}
