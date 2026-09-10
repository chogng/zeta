//! Bounded text display items owned by agent extensions, independent of tool execution.
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExtensionItemStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionItem {
    #[schemars(length(min = 1, max = 128))]
    pub extension: String,
    #[schemars(length(min = 1, max = 128))]
    pub id: String,
    #[schemars(length(min = 1, max = 512))]
    pub title: String,
    #[schemars(length(max = 131072))]
    pub body: String,
    pub status: ExtensionItemStatus,
}

impl ExtensionItem {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.extension.is_empty()
            || self.extension.len() > 128
            || self.id.is_empty()
            || self.id.len() > 128
            || self.title.is_empty()
            || self.title.len() > 512
            || self.body.len() > 131072
        {
            return Err("extension item exceeds its text bounds");
        }
        Ok(())
    }
}
