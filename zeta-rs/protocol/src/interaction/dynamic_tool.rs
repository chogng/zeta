use crate::{ToolCallId, ToolName};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolSpec {
    pub name: ToolName,
    pub description: String,
    #[ts(type = "unknown")]
    pub input_schema: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolCall {
    pub call_id: ToolCallId,
    pub name: ToolName,
    #[ts(type = "unknown")]
    pub arguments: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolResponse {
    pub call_id: ToolCallId,
    pub content: Vec<DynamicToolOutput>,
    pub success: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DynamicToolOutput {
    Text { text: String },
    Image { data_url: String },
}
