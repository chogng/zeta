use crate::{ApiProfile, BaseUrlNormalization, ProviderConfigError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use zeta_protocol::ProviderId;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProviderConfig {
    pub provider: ProviderId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

impl ModelProviderConfig {
    pub fn new(provider: ProviderId) -> Self {
        Self {
            provider,
            base_url: None,
            max_output_tokens: None,
        }
    }

    pub fn validate_static(&self) -> Result<(), ProviderConfigError> {
        if self.max_output_tokens == Some(0) {
            return Err(ProviderConfigError::InvalidMaxOutputTokens(
                self.provider.clone(),
            ));
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
