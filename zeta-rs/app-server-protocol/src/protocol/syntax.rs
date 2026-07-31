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

/// Protocol-owned syntax token categories referenced by compact token data.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SyntaxTokenTypeDto {
    Attribute,
    Comment,
    Constant,
    Constructor,
    Embedded,
    Function,
    Keyword,
    Label,
    Module,
    Number,
    Operator,
    Property,
    String,
    Type,
    Variable,
}

/// Zero-based UTF-16 position used by syntax-analysis protocol results.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxPositionDto {
    pub line: usize,
    pub character: usize,
}

/// Half-open UTF-16 range used by syntax-analysis protocol results.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxRangeDto {
    pub start: SyntaxPositionDto,
    pub end: SyntaxPositionDto,
}

/// Compact LSP-compatible token data and its protocol-owned legend.
///
/// Each group of five integers is `deltaLine, deltaStartUtf16, lengthUtf16, tokenType,
/// modifierBits`. `tokenType` indexes `legend`; modifier bits are currently zero.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxTokenDataDto {
    pub legend: Vec<SyntaxTokenTypeDto>,
    pub data: Vec<u32>,
}

/// Language-neutral kind for one syntactically declared document symbol.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SyntaxDocumentSymbolKindDto {
    Constant,
    Enum,
    Field,
    Function,
    Macro,
    Method,
    Module,
    Static,
    Struct,
    Trait,
    Type,
    Variable,
}

/// One syntactically declared symbol derived from an exact document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxDocumentSymbolDto {
    pub name: String,
    pub kind: SyntaxDocumentSymbolKindDto,
    pub range: SyntaxRangeDto,
    pub selection_range: SyntaxRangeDto,
}

/// Severity of one syntax parser diagnostic.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SyntaxDiagnosticSeverityDto {
    Error,
}

/// Recoverable parser error or missing construct for an exact document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxDiagnosticDto {
    pub range: SyntaxRangeDto,
    pub severity: SyntaxDiagnosticSeverityDto,
    pub message: String,
    pub source: String,
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

/// Complete presentation-independent syntax analysis derived from one exact document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxAnalysisSnapshotDto {
    pub revision: usize,
    pub result_id: String,
    pub has_errors: bool,
    pub tokens: SyntaxTokenDataDto,
    pub folding_ranges: Vec<SyntaxRangeDto>,
    pub symbols: Vec<SyntaxDocumentSymbolDto>,
    pub diagnostics: Vec<SyntaxDiagnosticDto>,
}
