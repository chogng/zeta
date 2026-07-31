use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Source language selected for one backend syntax-analysis document.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SyntaxLanguageDto {
    Json,
    Jsonc,
    Rust,
}

/// Opens or replaces one connection-owned incremental syntax-analysis document.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxOpenParams {
    #[schemars(length(min = 1, max = 256))]
    pub document_id: String,
    #[schemars(length(min = 1, max = 16384))]
    pub document_uri: String,
    pub language: SyntaxLanguageDto,
    pub revision: usize,
    pub text: String,
}

/// One replacement expressed in UTF-16 offsets against `previous_revision`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxTextEditDto {
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub text: String,
}

/// Applies one atomic, non-overlapping editor change event to an open document.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxChangeParams {
    #[schemars(length(min = 1, max = 256))]
    pub document_id: String,
    pub previous_revision: usize,
    pub revision: usize,
    #[schemars(length(min = 1, max = 1024))]
    pub edits: Vec<SyntaxTextEditDto>,
}

/// Closes one connection-owned syntax-analysis document.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxCloseParams {
    #[schemars(length(min = 1, max = 256))]
    pub document_id: String,
}

/// LSP-compatible relative semantic-token encoding derived from one exact syntax revision.
///
/// Each group of five integers is `deltaLine, deltaStartUtf16, lengthUtf16, tokenType,
/// modifierBits`. The fixed token-type legend is owned by the Desktop Alpha adapter.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxTokenSnapshotDto {
    pub revision: usize,
    pub result_id: String,
    pub data: Vec<u32>,
}
