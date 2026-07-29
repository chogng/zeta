use crate::protocol::common::CommandId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeta_protocol::SkillId;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SkillCatalogReloadDto {
    #[default]
    Cached,
    Refresh,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SkillEnablementDto {
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SkillSourceKindDto {
    BuiltIn,
    User,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SkillCompatibilityDto {
    Compatible,
    Unknown { note: String },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillDto {
    pub id: SkillId,
    pub description: String,
    pub source_kind: SkillSourceKindDto,
    pub content_digest: String,
    pub enablement: SkillEnablementDto,
    pub compatibility: SkillCompatibilityDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SkillDiagnosticCodeDto {
    SourceUnavailable,
    SourceLimitExceeded,
    SkillNotFound,
    InvalidFrontmatter,
    InvalidSkillName,
    DescriptionInvalid,
    PathEscapesRoot,
    UnsupportedFileType,
    ContentTooLarge,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillDiagnosticDto {
    pub source: String,
    pub subject: Option<String>,
    pub code: SkillDiagnosticCodeDto,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillListParams {
    #[serde(default)]
    pub reload: SkillCatalogReloadDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillListResult {
    #[ts(type = "number")]
    pub generation: u64,
    pub skills: Vec<SkillDto>,
    pub diagnostics: Vec<SkillDiagnosticDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillSetEnablementParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub skill_id: SkillId,
    pub enablement: SkillEnablementDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SkillsChanged {
    #[ts(type = "number")]
    pub generation: u64,
}
