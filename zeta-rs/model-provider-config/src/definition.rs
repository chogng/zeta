use crate::config::{is_http_url, normalize_base_url};
use crate::{ProviderConfigError, ProviderId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use zeta_protocol::Model;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderAdapter {
    OpenAi,
    OpenAiCompatible,
    Anthropic,
    Google,
    Xai,
    Qwen,
    Kimi,
    DeepSeek,
    Ollama,
    HuggingFace,
    Zai,
    MiniMax,
    Mimo,
}

/// A provider-independent model invocation API selected by a provider
/// definition.
///
/// This is a declarative value: `zeta-model-provider` resolves it to an
/// executable `zeta-api::ApiEndpoint` without making this configuration crate
/// depend on protocol codecs.
#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "camelCase")]
pub enum ApiProfile {
    OpenAiResponses,
    OpenAiChatCompletions,
    AnthropicMessages,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum EndpointPolicy {
    ProviderDefault { base_url: String },
    ConfiguredOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCatalogPolicy {
    ListedOnly,
    AllowUnlisted,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BaseUrlNormalization {
    Preserve,
    Trim,
    #[default]
    TrimAndRemoveTrailingSlash,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDefinition {
    pub id: ProviderId,
    pub name: String,
    pub adapter: ProviderAdapter,
    pub api_profile: ApiProfile,
    pub endpoint: EndpointPolicy,
    pub model_catalog_policy: ModelCatalogPolicy,
    #[serde(default)]
    pub models: Vec<Model>,
    #[serde(default)]
    pub defaults: ProviderDefaults,
    #[serde(default)]
    pub base_url_normalization: BaseUrlNormalization,
}

impl ProviderDefinition {
    pub fn new(
        id: ProviderId,
        name: impl Into<String>,
        adapter: ProviderAdapter,
        api_profile: ApiProfile,
        endpoint: EndpointPolicy,
        model_catalog_policy: ModelCatalogPolicy,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            adapter,
            api_profile,
            endpoint,
            model_catalog_policy,
            models: Vec::new(),
            defaults: ProviderDefaults::default(),
            base_url_normalization: BaseUrlNormalization::default(),
        }
    }

    pub fn with_models(mut self, models: impl IntoIterator<Item = Model>) -> Self {
        self.models.extend(models);
        self
    }

    pub fn with_defaults(mut self, defaults: ProviderDefaults) -> Self {
        self.defaults = defaults;
        self
    }

    pub fn with_base_url_normalization(mut self, rule: BaseUrlNormalization) -> Self {
        self.base_url_normalization = rule;
        self
    }

    pub fn validate(&self) -> Result<(), ProviderConfigError> {
        if self.name.trim().is_empty() {
            return Err(self.invalid("display name must not be empty"));
        }
        if let EndpointPolicy::ProviderDefault { base_url } = &self.endpoint
            && !is_http_url(&normalize_base_url(base_url, self.base_url_normalization))
        {
            return Err(ProviderConfigError::InvalidBaseUrl {
                provider: self.id.clone(),
                base_url: base_url.clone(),
            });
        }
        if self.defaults.max_output_tokens == Some(0) {
            return Err(ProviderConfigError::InvalidMaxOutputTokens(self.id.clone()));
        }
        let mut model_ids = BTreeSet::new();
        for model in &self.models {
            if !model_ids.insert(model.id.clone()) {
                return Err(self.invalid(format!("duplicate model ID '{}'", model.id)));
            }
            if model.display_name.trim().is_empty() {
                return Err(self.invalid(format!(
                    "model '{}' display name must not be empty",
                    model.id
                )));
            }
        }
        Ok(())
    }

    fn invalid(&self, message: impl Into<String>) -> ProviderConfigError {
        ProviderConfigError::InvalidProvider {
            provider: self.id.clone(),
            message: message.into(),
        }
    }
}
