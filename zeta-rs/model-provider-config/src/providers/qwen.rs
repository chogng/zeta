use super::default_provider;
use crate::{ApiProfile, Model, ModelId, ProviderAdapter, ProviderDefinition};

pub(super) fn definition() -> ProviderDefinition {
    default_provider(
        "qwen",
        "Qwen",
        ProviderAdapter::Qwen,
        ApiProfile::OpenAiChatCompletions,
        "https://dashscope.aliyuncs.com/compatible-mode/v1",
    )
    .with_models([Model::new(
        ModelId::new("qwen-plus").expect("valid model ID"),
        "Qwen Plus",
    )])
}
