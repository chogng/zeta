use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;
use zeta_protocol::Automation;
use zeta_protocol::AutomationDefinition;
use zeta_protocol::AutomationRun;
use zeta_protocol::AutomationStatus;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationListResult {
    pub automations: Vec<Automation>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationWriteParams {
    pub command_id: String,
    pub id: String,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub definition: AutomationDefinition,
    pub status: AutomationStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationDeleteParams {
    pub id: String,
    #[ts(type = "number")]
    pub expected_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunParams {
    pub id: String,
    pub command_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunsParams {
    pub id: String,
    pub limit: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRunsResult {
    pub runs: Vec<AutomationRun>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AutomationStopParams {
    pub run_id: String,
}
