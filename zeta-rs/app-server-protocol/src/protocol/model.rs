use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeta_protocol::ModelRef;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogEntry {
    pub model: ModelRef,
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResult {
    pub models: Vec<ModelCatalogEntry>,
}
