use crate::{ApiProfile, BaseUrlNormalization, ProviderConfigError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zeta_protocol::ModelId;
use zeta_protocol::ProviderId;

/// Model-specific context limits supplied by user or host configuration.
///
/// This metadata is deliberately separate from transport normalization: Core
/// consumes it to decide whether it can enforce a context budget itself.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelContextConfig {
    pub context_window: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_compact_token_limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProviderConfig {
    pub provider: ProviderId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub model_context: BTreeMap<ModelId, ModelContextConfig>,
}

impl ModelProviderConfig {
    pub fn new(provider: ProviderId) -> Self {
        Self {
            provider,
            base_url: None,
            max_output_tokens: None,
            model_context: BTreeMap::new(),
        }
    }

    pub fn validate_static(&self) -> Result<(), ProviderConfigError> {
        if self.max_output_tokens == Some(0) {
            return Err(ProviderConfigError::InvalidMaxOutputTokens(
                self.provider.clone(),
            ));
        }
        for (model, context) in &self.model_context {
            if context.context_window == 0 || context.auto_compact_token_limit == Some(0) {
                return Err(ProviderConfigError::InvalidModelContext {
                    provider: self.provider.clone(),
                    model: model.clone(),
                });
            }
        }
        if let Some(base_url) = self.base_url.as_deref() {
            let base_url = base_url.trim();
            if !base_url.is_empty() && !is_http_url(base_url) {
                return Err(ProviderConfigError::InvalidBaseUrl {
                    provider: self.provider.clone(),
                    base_url: base_url.into(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedModelProviderConfig {
    pub provider: ProviderId,
    pub api_profile: ApiProfile,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

pub(crate) fn normalize_base_url(value: &str, rule: BaseUrlNormalization) -> String {
    match rule {
        BaseUrlNormalization::Preserve => value.into(),
        BaseUrlNormalization::Trim => value.trim().into(),
        BaseUrlNormalization::TrimAndRemoveTrailingSlash => {
            value.trim().trim_end_matches('/').into()
        }
    }
}

pub(crate) fn is_http_url(value: &str) -> bool {
    let Some(authority_and_path) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = authority_and_path.split('/').next().unwrap_or_default();
    !authority.is_empty() && !authority.chars().any(char::is_whitespace)
}
