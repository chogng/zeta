use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigReadResult {
    pub preferred_model: Option<String>,
    pub theme: Option<ThemeDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdateParams {
    pub idempotency_key: String,
    pub preferred_model: Option<Option<String>>,
    pub theme: Option<Option<ThemeDto>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeDto {
    Light,
    Dark,
    System,
}
