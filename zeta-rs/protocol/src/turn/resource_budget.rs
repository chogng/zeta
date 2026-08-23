use crate::ModelRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Immutable USD price inputs used to evaluate one Turn's cost budget.
///
/// Rates are expressed as micro-USD per one million tokens so the durable contract never depends
/// on floating-point arithmetic or a mutable model catalog.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelPriceSnapshot {
    pub model: ModelRef,
    pub revision: String,
    #[ts(type = "number")]
    pub input_usd_micros_per_million_tokens: u64,
    #[ts(type = "number")]
    pub cached_input_usd_micros_per_million_tokens: u64,
    #[ts(type = "number")]
    pub output_usd_micros_per_million_tokens: u64,
}

/// Optional resource ceilings frozen when a Turn is accepted.
///
/// `max_total_tokens` counts provider-reported input plus output tokens. Cached input and
/// reasoning tokens remain observable usage subsets and are not counted twice. A cost ceiling
/// requires `price_snapshot`; Core validates both the snapshot revision and exact model binding.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TurnResourceBudget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number | null")]
    pub max_total_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number | null")]
    pub max_cost_usd_micros: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub price_snapshot: Option<ModelPriceSnapshot>,
}
