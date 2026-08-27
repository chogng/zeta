use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;
use zeroize::Zeroize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ProviderApiKeyPolicyDto {
    Unsupported,
    Optional,
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogEntryDto {
    pub provider: String,
    pub display_name: String,
    pub api_key_policy: ProviderApiKeyPolicyDto,
    pub api_key_configured: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderListResult {
    pub providers: Vec<ProviderCatalogEntryDto>,
}

/// Inbound-only provider API key that redacts diagnostics and clears its allocation on drop.
#[derive(Deserialize, Eq, JsonSchema, PartialEq, TS)]
#[serde(transparent)]
pub struct ProviderApiKeyDto(String);

impl ProviderApiKeyDto {
    pub fn into_bytes(mut self) -> Vec<u8> {
        std::mem::take(&mut self.0).into_bytes()
    }
}

impl fmt::Debug for ProviderApiKeyDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProviderApiKeyDto([REDACTED])")
    }
}

impl Drop for ProviderApiKeyDto {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Deserialize, Eq, JsonSchema, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderApiKeySetParams {
    pub provider: String,
    pub api_key: ProviderApiKeyDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProviderApiKeySetResult {
    pub provider: String,
    pub api_key_configured: bool,
}
