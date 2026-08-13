use crate::protocol::fs::{FsDeleteMode, FsExistingTargetBehavior, FsMissingTargetBehavior};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use ts_rs::TS;

/// Cross-file language operation requested for one source position.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageLocationKindDto {
    Declaration,
    Definition,
    Implementation,
    TypeDefinition,
    References,
}

/// One zero-based UTF-16 editor position.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePositionDto {
    pub line_index: u32,
    pub column_index: u32,
}

/// One ordered, end-exclusive UTF-16 editor range.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageRangeDto {
    pub start: LanguagePositionDto,
    pub end: LanguagePositionDto,
}

/// Authoritative source snapshot submitted before one language request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDocumentDto {
    pub path: PathBuf,
    pub language_id: String,
    #[ts(type = "number")]
    pub revision: u64,
    #[schemars(length(max = 10_485_760))]
    pub text: String,
}

/// Updates the language server with one authoritative editor document snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageSynchronizeParams {
    pub document: LanguageDocumentDto,
}

/// Releases one workspace document from the language-server session.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCloseParams {
    pub path: PathBuf,
}

/// Hover request against exactly one submitted document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageHoverParams {
    pub document: LanguageDocumentDto,
    pub position: LanguagePositionDto,
}

/// Fresh hover content expressed in editor-native UTF-16 coordinates.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageHoverResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub contents: Option<String>,
    pub range: Option<LanguageRangeDto>,
}

/// Why the editor requested completion candidates.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageCompletionTriggerKindDto {
    Invoke,
    TriggerCharacter,
    IncompleteRefresh,
}

/// Completion request against exactly one submitted document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCompletionsParams {
    pub document: LanguageDocumentDto,
    pub position: LanguagePositionDto,
    pub trigger_kind: LanguageCompletionTriggerKindDto,
    pub trigger_character: Option<String>,
}

/// Presentation-neutral completion category understood by editor products.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageCompletionItemKindDto {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    File,
    Folder,
    Reference,
    TypeParameter,
}

/// Whether completion insertion text is literal text or snippet syntax.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageCompletionInsertTextFormatDto {
    PlainText,
    Snippet,
}

/// One bounded completion candidate with one safe primary document edit.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCompletionItemDto {
    pub label: String,
    pub kind: LanguageCompletionItemKindDto,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
    pub preselect: Option<bool>,
    pub commit_characters: Vec<String>,
    pub insert_text_format: LanguageCompletionInsertTextFormatDto,
    pub range: LanguageRangeDto,
    pub insert_text: String,
}

/// Fresh completion candidates for exactly one source document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCompletionsResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub is_incomplete: bool,
    pub items: Vec<LanguageCompletionItemDto>,
}

/// Cross-file request against exactly one submitted document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageLocationsParams {
    pub document: LanguageDocumentDto,
    pub position: LanguagePositionDto,
    pub kind: LanguageLocationKindDto,
    #[serde(default)]
    pub include_declaration: bool,
}

/// One workspace-relative target returned in editor-native UTF-16 coordinates.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageLocationDto {
    pub path: PathBuf,
    pub range: LanguageRangeDto,
    pub selection_range: LanguageRangeDto,
}

/// Fresh cross-file targets for exactly one source document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageLocationsResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub locations: Vec<LanguageLocationDto>,
}

/// Call- or type-hierarchy operation requested by an editor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageHierarchyKindDto {
    PrepareCall,
    IncomingCalls,
    OutgoingCalls,
    PrepareType,
    Supertypes,
    Subtypes,
}

/// One hierarchy symbol, including opaque server data required for follow-up requests.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageHierarchyItemDto {
    pub name: String,
    pub symbol_kind: u32,
    pub detail: Option<String>,
    pub path: PathBuf,
    pub range: LanguageRangeDto,
    pub selection_range: LanguageRangeDto,
    #[ts(type = "unknown")]
    pub data: Option<Value>,
}

/// Hierarchy request against exactly one submitted document revision.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageHierarchyParams {
    pub document: LanguageDocumentDto,
    pub kind: LanguageHierarchyKindDto,
    pub position: Option<LanguagePositionDto>,
    pub item: Option<LanguageHierarchyItemDto>,
}

/// One hierarchy result and optional call-site ranges.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageHierarchyEntryDto {
    pub item: LanguageHierarchyItemDto,
    pub from_path: Option<PathBuf>,
    pub from_ranges: Vec<LanguageRangeDto>,
}

/// Fresh hierarchy entries for exactly one source document revision.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageHierarchyResultDto {
    #[ts(type = "number")]
    pub revision: u64,
    pub entries: Vec<LanguageHierarchyEntryDto>,
}

/// Project-wide symbol search for one language-server family.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageWorkspaceSymbolsParams {
    pub language_id: String,
    #[schemars(length(max = 1024))]
    pub query: String,
}

/// One project-wide symbol with an exact workspace location.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageWorkspaceSymbolDto {
    pub name: String,
    pub symbol_kind: u32,
    pub container_name: Option<String>,
    pub path: PathBuf,
    pub range: LanguageRangeDto,
}

/// Bounded project-wide symbol result.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageWorkspaceSymbolsResult {
    pub symbols: Vec<LanguageWorkspaceSymbolDto>,
}

/// Prepare-rename request against one exact source snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePrepareRenameParams {
    pub document: LanguageDocumentDto,
    pub position: LanguagePositionDto,
}

/// Rename target range and initial input text, or `None` when unavailable.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageRenamePreparationDto {
    pub range: LanguageRangeDto,
    pub placeholder: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePrepareRenameResult {
    pub preparation: Option<LanguageRenamePreparationDto>,
}

/// Workspace rename request against one exact source snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageRenameParams {
    pub document: LanguageDocumentDto,
    pub position: LanguagePositionDto,
    #[schemars(length(min = 1, max = 1024))]
    pub new_name: String,
}

/// One UTF-16 text replacement.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageTextEditDto {
    pub range: LanguageRangeDto,
    pub new_text: String,
}

/// Named editor preferences for one formatting request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageFormattingOptionsDto {
    pub tab_size: u32,
    pub insert_spaces: bool,
    pub trim_trailing_whitespace: Option<bool>,
}

/// Whole-document formatting against one exact editor snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDocumentFormattingParams {
    pub document: LanguageDocumentDto,
    pub options: LanguageFormattingOptionsDto,
}

/// Range formatting against one exact editor snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageRangeFormattingParams {
    pub document: LanguageDocumentDto,
    pub range: LanguageRangeDto,
    pub options: LanguageFormattingOptionsDto,
}

/// Validated UTF-16 formatting edits for the submitted snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageFormattingResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub edits: Vec<LanguageTextEditDto>,
}

/// Why the editor requested signature help.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageSignatureHelpTriggerKindDto {
    Invoke,
    TriggerCharacter,
    ContentChange,
}

/// Signature-help request against one exact editor snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageSignatureHelpParams {
    pub document: LanguageDocumentDto,
    pub position: LanguagePositionDto,
    pub trigger_kind: LanguageSignatureHelpTriggerKindDto,
    pub trigger_character: Option<String>,
}

/// One parameter in a callable signature.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageParameterInformationDto {
    pub label: String,
    pub documentation: Option<String>,
}

/// One callable signature returned by a language server.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageSignatureInformationDto {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<LanguageParameterInformationDto>,
    pub active_parameter: Option<u32>,
}

/// Fresh signature help for the submitted document snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageSignatureHelpResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub signatures: Vec<LanguageSignatureInformationDto>,
    pub active_signature: Option<u32>,
}

/// Inlay-hint request against one exact editor snapshot and UTF-16 range.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageInlayHintsParams {
    pub document: LanguageDocumentDto,
    pub range: LanguageRangeDto,
}

/// Presentation-neutral inlay-hint category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageInlayHintKindDto {
    Type,
    Parameter,
    Other,
}

/// One non-mutating inlay hint in editor-native UTF-16 coordinates.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageInlayHintDto {
    pub position: LanguagePositionDto,
    pub label: String,
    pub kind: LanguageInlayHintKindDto,
    pub tooltip: Option<String>,
    pub padding_left: bool,
    pub padding_right: bool,
}

/// Fresh bounded inlay hints for the submitted snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageInlayHintsResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub hints: Vec<LanguageInlayHintDto>,
}

/// Linked-editing request against one exact editor snapshot and UTF-16 position.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageLinkedEditingRangesParams {
    pub document: LanguageDocumentDto,
    pub position: LanguagePositionDto,
}

/// Fresh linked ranges whose text must remain identical in one editor transaction.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageLinkedEditingRangesResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub ranges: Vec<LanguageRangeDto>,
    pub word_pattern: Option<String>,
}

/// Text replacements for one exact workspace content baseline.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageTextDocumentEditDto {
    pub path: PathBuf,
    pub expected_text: String,
    pub edits: Vec<LanguageTextEditDto>,
}

/// One ordered text or resource operation returned by a language server.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(tag = "kind")]
pub enum LanguageWorkspaceEditEntryDto {
    TextDocument {
        document: LanguageTextDocumentEditDto,
    },
    Create {
        path: PathBuf,
        existing: FsExistingTargetBehavior,
    },
    Rename {
        source: PathBuf,
        target: PathBuf,
        existing: FsExistingTargetBehavior,
    },
    Delete {
        path: PathBuf,
        missing: FsMissingTargetBehavior,
        mode: FsDeleteMode,
    },
}

/// Ordered workspace edit spanning text documents and workspace resources.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageWorkspaceEditDto {
    pub entries: Vec<LanguageWorkspaceEditEntryDto>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum LanguageDiagnosticSeverityDto {
    Error,
    Warning,
    Information,
    Hint,
}

/// Diagnostic context submitted with a code-action request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCodeActionDiagnosticDto {
    pub range: LanguageRangeDto,
    pub severity: LanguageDiagnosticSeverityDto,
    pub message: String,
    #[ts(type = "unknown")]
    pub code: Option<Value>,
    pub source: Option<String>,
}

/// Fresh language-server diagnostics for one exact workspace document revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDiagnosticsNotification {
    pub path: PathBuf,
    #[ts(type = "number")]
    pub revision: u64,
    pub diagnostics: Vec<LanguageCodeActionDiagnosticDto>,
}

/// Code-action request against one exact source snapshot and selection.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCodeActionsParams {
    pub document: LanguageDocumentDto,
    pub range: LanguageRangeDto,
    pub diagnostics: Vec<LanguageCodeActionDiagnosticDto>,
    pub only: Vec<String>,
}

/// One code action. `provider_data` must be returned unchanged for resolution.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCodeActionDto {
    pub title: String,
    pub kind: Option<String>,
    pub is_preferred: bool,
    pub disabled_reason: Option<String>,
    pub edit: Option<LanguageWorkspaceEditDto>,
    #[ts(type = "unknown")]
    pub provider_data: Value,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCodeActionsResult {
    pub actions: Vec<LanguageCodeActionDto>,
}

/// Resolve request for one action returned by the same language server.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LanguageResolveCodeActionParams {
    pub document: LanguageDocumentDto,
    #[ts(type = "unknown")]
    pub provider_data: Value,
}
