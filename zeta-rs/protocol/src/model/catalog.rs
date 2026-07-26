use crate::Personality;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use ts_rs::TS;

macro_rules! model_identifier {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize, TS)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidModelIdentity> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(InvalidModelIdentity(
                        concat!($label, " must not be empty").into(),
                    ));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

model_identifier!(ProviderId, "provider ID");
model_identifier!(ModelId, "model ID");

#[derive(Clone, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    pub provider: ProviderId,
    pub model: ModelId,
}

impl ModelRef {
    pub fn new(provider: ProviderId, model: ModelId) -> Self {
        Self { provider, model }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    ExtraHigh,
    Max,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CapabilitySupport {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ContextWindow {
    Known(u32),
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    pub tools: CapabilitySupport,
    pub reasoning: CapabilitySupport,
    pub parallel_tool_calls: CapabilitySupport,
    pub personality: CapabilitySupport,
}

impl ModelCapabilities {
    pub const UNKNOWN: Self = Self {
        tools: CapabilitySupport::Unknown,
        reasoning: CapabilitySupport::Unknown,
        parallel_tool_calls: CapabilitySupport::Unknown,
        personality: CapabilitySupport::Unknown,
    };
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: ModelId,
    pub display_name: String,
    pub context_window: ContextWindow,
    pub auto_compact_token_limit: Option<u32>,
    pub capabilities: ModelCapabilities,
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    pub default_reasoning_effort: Option<ReasoningEffort>,
    pub default_personality: Option<Personality>,
}

impl ModelInfo {
    pub fn new(id: ModelId, display_name: impl Into<String>) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            context_window: ContextWindow::Unknown,
            auto_compact_token_limit: None,
            capabilities: ModelCapabilities::UNKNOWN,
            supported_reasoning_efforts: Vec::new(),
            default_reasoning_effort: None,
            default_personality: None,
        }
    }

    pub fn effective_auto_compact_token_limit(&self) -> Option<u32> {
        let context_limit = match self.context_window {
            ContextWindow::Known(tokens) => Some(tokens.saturating_mul(9) / 10),
            ContextWindow::Unknown => None,
        };
        match (context_limit, self.auto_compact_token_limit) {
            (Some(context), Some(configured)) => Some(context.min(configured)),
            (Some(context), None) => Some(context),
            (None, configured) => configured,
        }
    }
}

pub type Model = ModelInfo;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreset {
    pub id: String,
    pub name: String,
    pub model: ModelRef,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub personality: Option<Personality>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidModelIdentity(pub String);

impl fmt::Display for InvalidModelIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for InvalidModelIdentity {}
