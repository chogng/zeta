use super::default_provider;
use crate::{ApiProfile, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "qwen",
        "Qwen",
        ProviderAdapter::Qwen,
        ApiProfile::OpenAiChatCompletions,
        "https://dashscope.aliyuncs.com/compatible-mode/v1",
    )
}
