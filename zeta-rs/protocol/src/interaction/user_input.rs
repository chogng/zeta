use crate::SkillRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A provider-independent input supplied by the user for one turn.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UserInput {
    Text { text: String },
    Image { url: String },
    LocalImage { path: String },
    Skill { skill: SkillRef },
    Mention { name: String, path: String },
}
