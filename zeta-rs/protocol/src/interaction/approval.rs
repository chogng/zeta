use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Capability kind displayed and approved through a durable Agent interaction.
#[derive(
    Clone, Copy, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize, TS,
)]
#[serde(rename_all = "camelCase")]
pub enum ActionApprovalCapabilityKind {
    FileRead,
    FileWrite,
    ProcessSpawn,
    Network,
    CredentialUse,
    ExternalMutation,
    SystemConfiguration,
    UserInterface,
}

/// One exact capability included in an action approval request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ActionApprovalCapability {
    pub kind: ActionApprovalCapabilityKind,
    #[schemars(length(min = 1))]
    pub scope: String,
}

/// Durable request for approval of one policy-bound action.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ActionApprovalRequest {
    #[schemars(length(min = 1))]
    pub action_digest: String,
    #[schemars(length(min = 1))]
    pub policy_revision: String,
    #[schemars(length(min = 1))]
    pub capabilities: Vec<ActionApprovalCapability>,
    #[schemars(length(min = 1))]
    pub reason: String,
}

/// User decision for one exact action approval interaction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ActionApprovalDecision {
    ApproveOnce,
    Decline,
}

/// Durable response to an action approval request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ActionApprovalResponse {
    pub decision: ActionApprovalDecision,
}
