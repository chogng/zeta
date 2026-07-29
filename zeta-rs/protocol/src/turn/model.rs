use crate::{ModelRef, PendingInteraction, StableTurnError, ThreadItem, TurnId, TurnStatus};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Canonical readable state for one user-intent-driven Agent execution.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub turn_id: TurnId,
    pub status: TurnStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub model: Option<ModelRef>,
    pub items: Vec<ThreadItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub pending_interaction: Option<PendingInteraction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub error: Option<StableTurnError>,
}
