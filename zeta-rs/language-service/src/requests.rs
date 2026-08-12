//! Product-neutral language request inputs and projected results.

use std::path::PathBuf;

use serde_json::Value;
use zeta_lsp::lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CodeAction,
    CodeActionOrCommand, CompletionItem, CompletionResponse, CompletionTextEdit,
    DocumentChangeOperation, DocumentChanges, Documentation, GotoDefinitionResponse, Hover,
    HoverContents, InsertTextFormat, LanguageString, Location, MarkedString, MarkupContent, OneOf,
    Position, PositionEncodingKind, PrepareRenameResponse, Range, ResourceOp, SymbolKind,
    TextDocumentEdit, TextEdit, TypeHierarchyItem, Uri, WorkspaceEdit, WorkspaceSymbolResponse,
};

use crate::projection::{byte_offset_for_position, byte_range_for_lsp_range};
use crate::{LanguageDocumentRevision, LanguageTextRange};

const MAX_COMPLETION_ITEMS: usize = 200;

/// Monotonic identity assigned when a product request crosses the service boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LanguageRequestId(u64);

impl LanguageRequestId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Request operation used for capability failures and asynchronous error reporting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageRequestKind {
    Hover,
    Completion,
    Declaration,
    Definition,
    Implementation,
    TypeDefinition,
    References,
    PrepareCallHierarchy,
    IncomingCalls,
    OutgoingCalls,
    PrepareTypeHierarchy,
    Supertypes,
    Subtypes,
    WorkspaceSymbols,
    PrepareRename,
    Rename,
    CodeActions,
    ResolveCodeAction,
}

/// UTF-8 position inside one source row of an authoritative editor snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageDocumentPosition {
    pub row: u32,
    pub byte_offset: u32,
}

impl LanguageDocumentPosition {
    pub const fn new(row: u32, byte_offset: u32) -> Self {
        Self { row, byte_offset }
    }
}

/// Fresh hover content bound to the exact document revision that requested it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageHover {
    pub request_id: LanguageRequestId,
    pub path: PathBuf,
    pub revision: LanguageDocumentRevision,
    pub contents: String,
    pub range: Option<LanguageTextRange>,
}

/// One bounded, presentation-neutral completion candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageCompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: String,
    pub edit: Option<LanguageTextEdit>,
}

/// One exact UTF-8 edit that can be safely delegated to an editor document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageTextEdit {
    pub range: LanguageTextRange,
    pub new_text: String,
}

/// Fresh completion candidates bound to the exact requesting revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageCompletions {
    pub request_id: LanguageRequestId,
    pub path: PathBuf,
    pub revision: LanguageDocumentRevision,
    pub is_incomplete: bool,
    pub items: Vec<LanguageCompletionItem>,
}

/// Encoding retained for a definition target whose text is not owned by this service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguagePositionEncoding {
    Utf8,
    Utf16,
}

/// Semantic operation that produced one set of cross-file locations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageLocationKind {
    Declaration,
    Definition,
    Implementation,
    TypeDefinition,
    Reference,
}

/// One target position expressed in the negotiated language-server encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageLocationPosition {
    pub row: u32,
    pub character: u32,
}

/// One ordered target range expressed in the negotiated language-server encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageLocationRange {
    pub start: LanguageLocationPosition,
    pub end: LanguageLocationPosition,
}

/// One filesystem target returned by a cross-file language request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageLocationTarget {
    pub path: PathBuf,
    pub range: LanguageLocationRange,
    pub selection_range: LanguageLocationRange,
    pub encoding: LanguagePositionEncoding,
}

/// Fresh cross-file targets bound to the exact requesting revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageLocations {
    pub request_id: LanguageRequestId,
    pub kind: LanguageLocationKind,
    pub source_path: PathBuf,
    pub source_revision: LanguageDocumentRevision,
    pub targets: Vec<LanguageLocationTarget>,
}

/// Semantic operation that produced one hierarchy result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageHierarchyKind {
    PrepareCall,
    IncomingCalls,
    OutgoingCalls,
    PrepareType,
    Supertypes,
    Subtypes,
}

/// One call- or type-hierarchy symbol with the server-owned data required for follow-up requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageHierarchyItem {
    pub name: String,
    pub symbol_kind: u32,
    pub detail: Option<String>,
    pub path: PathBuf,
    pub range: LanguageLocationRange,
    pub selection_range: LanguageLocationRange,
    pub encoding: LanguagePositionEncoding,
    pub data: Option<Value>,
}

/// One hierarchy edge. `from_ranges` identifies call sites and is empty for type edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageHierarchyEntry {
    pub item: LanguageHierarchyItem,
    pub from_path: Option<PathBuf>,
    pub from_ranges: Vec<LanguageLocationRange>,
}

/// Fresh hierarchy entries bound to the exact source revision that initiated the operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageHierarchyResult {
    pub request_id: LanguageRequestId,
    pub kind: LanguageHierarchyKind,
    pub source_path: PathBuf,
    pub source_revision: LanguageDocumentRevision,
    pub entries: Vec<LanguageHierarchyEntry>,
}

/// One project-wide symbol returned independently of an open editor document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageWorkspaceSymbol {
    pub name: String,
    pub symbol_kind: u32,
    pub container_name: Option<String>,
    pub path: PathBuf,
    pub range: LanguageLocationRange,
    pub encoding: LanguagePositionEncoding,
}

/// Bounded project-wide symbol response from one language server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageWorkspaceSymbols {
    pub request_id: LanguageRequestId,
    pub query: String,
    pub symbols: Vec<LanguageWorkspaceSymbol>,
}

/// One prepare-rename response projected into the authoritative source snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageRenamePreparation {
    pub request_id: LanguageRequestId,
    pub source_path: PathBuf,
    pub source_revision: LanguageDocumentRevision,
    pub range: Option<LanguageTextRange>,
    pub placeholder: Option<String>,
    pub default_behavior: bool,
}

/// One protocol-encoded text replacement for a workspace resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageWorkspaceTextEdit {
    pub range: LanguageLocationRange,
    pub new_text: String,
}

/// All text replacements for one resource in a language-server workspace edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageWorkspaceDocumentEdit {
    pub path: PathBuf,
    pub server_version: Option<i32>,
    pub edits: Vec<LanguageWorkspaceTextEdit>,
}

/// Behavior when a workspace-edit create or rename target already exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageExistingTargetBehavior {
    Error,
    Overwrite,
    Ignore,
}

/// Behavior when a workspace-edit delete target does not exist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageMissingTargetBehavior {
    Error,
    Ignore,
}

/// Scope of one workspace-edit delete operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageDeleteMode {
    FileOrEmptyDirectory,
    Recursive,
}

/// One resource operation in the exact order supplied by the language server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageWorkspaceEditEntry {
    TextDocument(LanguageWorkspaceDocumentEdit),
    Create {
        path: PathBuf,
        existing: LanguageExistingTargetBehavior,
    },
    Rename {
        source: PathBuf,
        target: PathBuf,
        existing: LanguageExistingTargetBehavior,
    },
    Delete {
        path: PathBuf,
        missing: LanguageMissingTargetBehavior,
        mode: LanguageDeleteMode,
    },
}

/// Ordered workspace edit containing text and optional resource operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageWorkspaceEdit {
    pub encoding: LanguagePositionEncoding,
    pub entries: Vec<LanguageWorkspaceEditEntry>,
}

/// Fresh rename result bound to the source revision that initiated it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageWorkspaceEditResult {
    pub request_id: LanguageRequestId,
    pub source_path: PathBuf,
    pub source_revision: LanguageDocumentRevision,
    pub edit: LanguageWorkspaceEdit,
}

/// One LSP code action with any text edit projected and resolve payload kept opaque.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageCodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub is_preferred: bool,
    pub disabled_reason: Option<String>,
    pub edit: Option<LanguageWorkspaceEdit>,
    pub provider_data: Value,
}

/// Fresh code actions bound to the source revision that requested them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageCodeActions {
    pub request_id: LanguageRequestId,
    pub source_path: PathBuf,
    pub source_revision: LanguageDocumentRevision,
    pub actions: Vec<LanguageCodeAction>,
}

pub(crate) fn protocol_position(
    text: &str,
    position: LanguageDocumentPosition,
    encoding: &PositionEncodingKind,
) -> Option<Position> {
    let row = usize::try_from(position.row).ok()?;
    let byte_offset = usize::try_from(position.byte_offset).ok()?;
    let line = source_line(text, row)?;
    if byte_offset > line.len() || !line.is_char_boundary(byte_offset) {
        return None;
    }
    let character = if *encoding == PositionEncodingKind::UTF8 {
        byte_offset
    } else if *encoding == PositionEncodingKind::UTF16 {
        line[..byte_offset].encode_utf16().count()
    } else {
        return None;
    };
    Some(Position::new(
        u32::try_from(row).ok()?,
        u32::try_from(character).ok()?,
    ))
}

pub(crate) fn project_hover(
    request_id: LanguageRequestId,
    path: PathBuf,
    revision: LanguageDocumentRevision,
    text: &str,
    encoding: &PositionEncodingKind,
    hover: Option<Hover>,
) -> Option<LanguageHover> {
    let hover = hover?;
    let contents = hover_contents(hover.contents);
    if contents.trim().is_empty() {
        return None;
    }
    let range = hover.range.and_then(|range| {
        byte_range_for_lsp_range(text, range.start, range.end, encoding).map(LanguageTextRange::new)
    });
    Some(LanguageHover {
        request_id,
        path,
        revision,
        contents,
        range,
    })
}

pub(crate) fn project_completions(
    request_id: LanguageRequestId,
    path: PathBuf,
    revision: LanguageDocumentRevision,
    request_position: LanguageDocumentPosition,
    text: &str,
    encoding: &PositionEncodingKind,
    response: Option<CompletionResponse>,
) -> LanguageCompletions {
    let (is_incomplete, items) = match response {
        Some(CompletionResponse::Array(items)) => (false, items),
        Some(CompletionResponse::List(list)) => (list.is_incomplete, list.items),
        None => (false, Vec::new()),
    };
    LanguageCompletions {
        request_id,
        path,
        revision,
        is_incomplete,
        items: items
            .into_iter()
            .take(MAX_COMPLETION_ITEMS)
            .map(|item| project_completion_item(item, request_position, text, encoding))
            .collect(),
    }
}

pub(crate) fn project_locations(
    request_id: LanguageRequestId,
    kind: LanguageLocationKind,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    response: Option<GotoDefinitionResponse>,
) -> LanguageLocations {
    let ranges = match response {
        Some(GotoDefinitionResponse::Scalar(location)) => {
            vec![(location.uri, location.range, location.range)]
        }
        Some(GotoDefinitionResponse::Array(locations)) => locations
            .into_iter()
            .map(|location| (location.uri, location.range, location.range))
            .collect(),
        Some(GotoDefinitionResponse::Link(links)) => links
            .into_iter()
            .map(|link| {
                (
                    link.target_uri,
                    link.target_range,
                    link.target_selection_range,
                )
            })
            .collect(),
        None => Vec::new(),
    };
    project_location_ranges(
        request_id,
        kind,
        source_path,
        source_revision,
        encoding,
        ranges,
    )
}

pub(crate) fn project_references(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    response: Option<Vec<Location>>,
) -> LanguageLocations {
    let ranges = response
        .unwrap_or_default()
        .into_iter()
        .map(|location| (location.uri, location.range, location.range))
        .collect();
    project_location_ranges(
        request_id,
        LanguageLocationKind::Reference,
        source_path,
        source_revision,
        encoding,
        ranges,
    )
}

pub(crate) fn project_call_hierarchy_items(
    request_id: LanguageRequestId,
    kind: LanguageHierarchyKind,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    items: Vec<CallHierarchyItem>,
) -> LanguageHierarchyResult {
    project_hierarchy_entries(
        request_id,
        kind,
        source_path,
        source_revision,
        items
            .into_iter()
            .filter_map(|item| {
                project_call_item(item, encoding).map(|item| LanguageHierarchyEntry {
                    item,
                    from_path: None,
                    from_ranges: Vec::new(),
                })
            })
            .collect(),
    )
}

pub(crate) fn project_incoming_calls(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    calls: Vec<CallHierarchyIncomingCall>,
) -> LanguageHierarchyResult {
    project_hierarchy_entries(
        request_id,
        LanguageHierarchyKind::IncomingCalls,
        source_path,
        source_revision,
        calls
            .into_iter()
            .filter_map(|call| {
                project_call_item(call.from, encoding).map(|item| LanguageHierarchyEntry {
                    from_path: Some(item.path.clone()),
                    item,
                    from_ranges: call.from_ranges.into_iter().map(location_range).collect(),
                })
            })
            .collect(),
    )
}

pub(crate) fn project_outgoing_calls(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    from_path: PathBuf,
    calls: Vec<CallHierarchyOutgoingCall>,
) -> LanguageHierarchyResult {
    project_hierarchy_entries(
        request_id,
        LanguageHierarchyKind::OutgoingCalls,
        source_path,
        source_revision,
        calls
            .into_iter()
            .filter_map(|call| {
                project_call_item(call.to, encoding).map(|item| LanguageHierarchyEntry {
                    item,
                    from_path: Some(from_path.clone()),
                    from_ranges: call.from_ranges.into_iter().map(location_range).collect(),
                })
            })
            .collect(),
    )
}

pub(crate) fn project_type_hierarchy_items(
    request_id: LanguageRequestId,
    kind: LanguageHierarchyKind,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    items: Vec<TypeHierarchyItem>,
) -> LanguageHierarchyResult {
    project_hierarchy_entries(
        request_id,
        kind,
        source_path,
        source_revision,
        items
            .into_iter()
            .filter_map(|item| {
                project_type_item(item, encoding).map(|item| LanguageHierarchyEntry {
                    item,
                    from_path: None,
                    from_ranges: Vec::new(),
                })
            })
            .collect(),
    )
}

pub(crate) fn protocol_call_hierarchy_item(
    item: LanguageHierarchyItem,
) -> Option<CallHierarchyItem> {
    Some(CallHierarchyItem {
        name: item.name,
        kind: symbol_kind(item.symbol_kind)?,
        tags: None,
        detail: item.detail,
        uri: file_uri(&item.path)?,
        range: protocol_location_range(item.range),
        selection_range: protocol_location_range(item.selection_range),
        data: item.data,
    })
}

pub(crate) fn protocol_type_hierarchy_item(
    item: LanguageHierarchyItem,
) -> Option<TypeHierarchyItem> {
    Some(TypeHierarchyItem {
        name: item.name,
        kind: symbol_kind(item.symbol_kind)?,
        tags: None,
        detail: item.detail,
        uri: file_uri(&item.path)?,
        range: protocol_location_range(item.range),
        selection_range: protocol_location_range(item.selection_range),
        data: item.data,
    })
}

pub(crate) fn project_workspace_symbols(
    request_id: LanguageRequestId,
    query: String,
    encoding: &PositionEncodingKind,
    response: Option<WorkspaceSymbolResponse>,
) -> LanguageWorkspaceSymbols {
    let symbols = match response {
        Some(WorkspaceSymbolResponse::Flat(symbols)) => symbols
            .into_iter()
            .filter_map(|symbol| {
                project_workspace_symbol(
                    symbol.name,
                    symbol.kind,
                    symbol.container_name,
                    symbol.location.uri,
                    symbol.location.range,
                    encoding,
                )
            })
            .collect(),
        Some(WorkspaceSymbolResponse::Nested(symbols)) => symbols
            .into_iter()
            .filter_map(|symbol| match symbol.location {
                OneOf::Left(location) => project_workspace_symbol(
                    symbol.name,
                    symbol.kind,
                    symbol.container_name,
                    location.uri,
                    location.range,
                    encoding,
                ),
                OneOf::Right(_) => None,
            })
            .collect(),
        None => Vec::new(),
    };
    LanguageWorkspaceSymbols {
        request_id,
        query,
        symbols,
    }
}

pub(crate) fn project_rename_preparation(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    text: &str,
    encoding: &PositionEncodingKind,
    response: Option<PrepareRenameResponse>,
) -> Option<LanguageRenamePreparation> {
    let (range, placeholder, default_behavior) = match response? {
        PrepareRenameResponse::Range(range) => (
            byte_range_for_lsp_range(text, range.start, range.end, encoding)
                .map(LanguageTextRange::new),
            None,
            false,
        ),
        PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } => (
            byte_range_for_lsp_range(text, range.start, range.end, encoding)
                .map(LanguageTextRange::new),
            Some(placeholder),
            false,
        ),
        PrepareRenameResponse::DefaultBehavior { default_behavior } => {
            (None, None, default_behavior)
        }
    };
    if range.is_none() && !default_behavior {
        return None;
    }
    Some(LanguageRenamePreparation {
        request_id,
        source_path,
        source_revision,
        range,
        placeholder,
        default_behavior,
    })
}

pub(crate) fn project_workspace_edit(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    edit: Option<WorkspaceEdit>,
) -> Result<LanguageWorkspaceEditResult, String> {
    let edit = project_text_workspace_edit(encoding, edit.unwrap_or_default())?;
    Ok(LanguageWorkspaceEditResult {
        request_id,
        source_path,
        source_revision,
        edit,
    })
}

pub(crate) fn project_code_actions(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    can_resolve: bool,
    response: Option<Vec<CodeActionOrCommand>>,
) -> LanguageCodeActions {
    let actions = response
        .unwrap_or_default()
        .into_iter()
        .filter_map(|candidate| match candidate {
            CodeActionOrCommand::Command(_) => None,
            CodeActionOrCommand::CodeAction(action) => {
                project_code_action(encoding, can_resolve, action).ok()
            }
        })
        .collect();
    LanguageCodeActions {
        request_id,
        source_path,
        source_revision,
        actions,
    }
}

pub(crate) fn project_resolved_code_action(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    action: CodeAction,
) -> Result<LanguageCodeActions, String> {
    Ok(LanguageCodeActions {
        request_id,
        source_path,
        source_revision,
        actions: vec![project_code_action(encoding, true, action)?],
    })
}

pub(crate) fn protocol_code_action(data: Value) -> Result<CodeAction, String> {
    serde_json::from_value(data)
        .map_err(|error| format!("invalid code-action resolve payload: {error}"))
}

fn project_code_action(
    encoding: &PositionEncodingKind,
    can_resolve: bool,
    action: CodeAction,
) -> Result<LanguageCodeAction, String> {
    let provider_data = serde_json::to_value(&action).map_err(|error| error.to_string())?;
    let command_only = action.command.is_some();
    let requires_unsupported_resolve = action.edit.is_none() && !command_only && !can_resolve;
    let disabled_reason = action
        .disabled
        .map(|disabled| disabled.reason)
        .or_else(|| {
            command_only
                .then(|| "This action requires an unsupported language-server command".into())
        })
        .or_else(|| {
            requires_unsupported_resolve
                .then(|| "This action requires unsupported language-server resolution".into())
        });
    let edit = action
        .edit
        .map(|edit| project_text_workspace_edit(encoding, edit))
        .transpose()?;
    Ok(LanguageCodeAction {
        title: action.title,
        kind: action.kind.map(|kind| kind.as_str().to_owned()),
        is_preferred: action.is_preferred.unwrap_or(false),
        disabled_reason,
        edit,
        provider_data,
    })
}

fn project_text_workspace_edit(
    encoding: &PositionEncodingKind,
    edit: WorkspaceEdit,
) -> Result<LanguageWorkspaceEdit, String> {
    let has_document_changes = edit.document_changes.is_some();
    let mut entries = if let Some(changes) = edit.document_changes {
        match changes {
            DocumentChanges::Edits(edits) => edits
                .into_iter()
                .map(|edit| {
                    project_text_document_edit(edit).map(LanguageWorkspaceEditEntry::TextDocument)
                })
                .collect::<Result<Vec<_>, _>>()?,
            DocumentChanges::Operations(operations) => operations
                .into_iter()
                .map(|operation| match operation {
                    DocumentChangeOperation::Edit(edit) => project_text_document_edit(edit)
                        .map(LanguageWorkspaceEditEntry::TextDocument),
                    DocumentChangeOperation::Op(ResourceOp::Create(operation)) => {
                        Ok(LanguageWorkspaceEditEntry::Create {
                            path: file_path(&operation.uri).ok_or_else(|| {
                                "workspace create target is not a file URI".to_owned()
                            })?,
                            existing: existing_target_behavior(
                                operation
                                    .options
                                    .as_ref()
                                    .and_then(|options| options.overwrite),
                                operation
                                    .options
                                    .as_ref()
                                    .and_then(|options| options.ignore_if_exists),
                            ),
                        })
                    }
                    DocumentChangeOperation::Op(ResourceOp::Rename(operation)) => {
                        Ok(LanguageWorkspaceEditEntry::Rename {
                            source: file_path(&operation.old_uri).ok_or_else(|| {
                                "workspace rename source is not a file URI".to_owned()
                            })?,
                            target: file_path(&operation.new_uri).ok_or_else(|| {
                                "workspace rename target is not a file URI".to_owned()
                            })?,
                            existing: existing_target_behavior(
                                operation
                                    .options
                                    .as_ref()
                                    .and_then(|options| options.overwrite),
                                operation
                                    .options
                                    .as_ref()
                                    .and_then(|options| options.ignore_if_exists),
                            ),
                        })
                    }
                    DocumentChangeOperation::Op(ResourceOp::Delete(operation)) => {
                        Ok(LanguageWorkspaceEditEntry::Delete {
                            path: file_path(&operation.uri).ok_or_else(|| {
                                "workspace delete target is not a file URI".to_owned()
                            })?,
                            missing: if operation
                                .options
                                .as_ref()
                                .and_then(|options| options.ignore_if_not_exists)
                                == Some(true)
                            {
                                LanguageMissingTargetBehavior::Ignore
                            } else {
                                LanguageMissingTargetBehavior::Error
                            },
                            mode: if operation
                                .options
                                .as_ref()
                                .and_then(|options| options.recursive)
                                == Some(true)
                            {
                                LanguageDeleteMode::Recursive
                            } else {
                                LanguageDeleteMode::FileOrEmptyDirectory
                            },
                        })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
        }
    } else {
        edit.changes
            .unwrap_or_default()
            .into_iter()
            .map(|(uri, edits)| {
                Ok(LanguageWorkspaceEditEntry::TextDocument(
                    LanguageWorkspaceDocumentEdit {
                        path: file_path(&uri)
                            .ok_or_else(|| "workspace edit target is not a file URI".to_owned())?,
                        server_version: None,
                        edits: edits.into_iter().map(project_workspace_text_edit).collect(),
                    },
                ))
            })
            .collect::<Result<Vec<_>, String>>()?
    };
    if !has_document_changes {
        entries.sort_by(|left, right| workspace_entry_path(left).cmp(workspace_entry_path(right)));
    }
    Ok(LanguageWorkspaceEdit {
        encoding: language_position_encoding(encoding),
        entries,
    })
}

fn existing_target_behavior(
    overwrite: Option<bool>,
    ignore: Option<bool>,
) -> LanguageExistingTargetBehavior {
    if overwrite == Some(true) {
        LanguageExistingTargetBehavior::Overwrite
    } else if ignore == Some(true) {
        LanguageExistingTargetBehavior::Ignore
    } else {
        LanguageExistingTargetBehavior::Error
    }
}

fn workspace_entry_path(entry: &LanguageWorkspaceEditEntry) -> &PathBuf {
    match entry {
        LanguageWorkspaceEditEntry::TextDocument(edit) => &edit.path,
        LanguageWorkspaceEditEntry::Create { path, .. }
        | LanguageWorkspaceEditEntry::Delete { path, .. } => path,
        LanguageWorkspaceEditEntry::Rename { source, .. } => source,
    }
}

fn project_text_document_edit(
    edit: TextDocumentEdit,
) -> Result<LanguageWorkspaceDocumentEdit, String> {
    Ok(LanguageWorkspaceDocumentEdit {
        path: file_path(&edit.text_document.uri)
            .ok_or_else(|| "workspace edit target is not a file URI".to_owned())?,
        server_version: edit.text_document.version,
        edits: edit
            .edits
            .into_iter()
            .map(|edit| match edit {
                OneOf::Left(edit) => project_workspace_text_edit(edit),
                OneOf::Right(edit) => project_workspace_text_edit(edit.text_edit),
            })
            .collect(),
    })
}

fn project_workspace_text_edit(edit: TextEdit) -> LanguageWorkspaceTextEdit {
    LanguageWorkspaceTextEdit {
        range: location_range(edit.range),
        new_text: edit.new_text,
    }
}

fn project_workspace_symbol(
    name: String,
    kind: SymbolKind,
    container_name: Option<String>,
    uri: Uri,
    range: Range,
    encoding: &PositionEncodingKind,
) -> Option<LanguageWorkspaceSymbol> {
    Some(LanguageWorkspaceSymbol {
        name,
        symbol_kind: serde_json::to_value(kind)
            .ok()?
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())?,
        container_name,
        path: file_path(&uri)?,
        range: location_range(range),
        encoding: language_position_encoding(encoding),
    })
}

fn project_hierarchy_entries(
    request_id: LanguageRequestId,
    kind: LanguageHierarchyKind,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    entries: Vec<LanguageHierarchyEntry>,
) -> LanguageHierarchyResult {
    LanguageHierarchyResult {
        request_id,
        kind,
        source_path,
        source_revision,
        entries,
    }
}

fn project_call_item(
    item: CallHierarchyItem,
    encoding: &PositionEncodingKind,
) -> Option<LanguageHierarchyItem> {
    project_hierarchy_item(
        item.name,
        item.kind,
        item.detail,
        item.uri,
        item.range,
        item.selection_range,
        item.data,
        encoding,
    )
}

fn project_type_item(
    item: TypeHierarchyItem,
    encoding: &PositionEncodingKind,
) -> Option<LanguageHierarchyItem> {
    project_hierarchy_item(
        item.name,
        item.kind,
        item.detail,
        item.uri,
        item.range,
        item.selection_range,
        item.data,
        encoding,
    )
}

fn project_hierarchy_item(
    name: String,
    kind: SymbolKind,
    detail: Option<String>,
    uri: Uri,
    range: Range,
    selection_range: Range,
    data: Option<Value>,
    encoding: &PositionEncodingKind,
) -> Option<LanguageHierarchyItem> {
    Some(LanguageHierarchyItem {
        name,
        symbol_kind: serde_json::to_value(kind)
            .ok()?
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())?,
        detail,
        path: file_path(&uri)?,
        range: location_range(range),
        selection_range: location_range(selection_range),
        encoding: language_position_encoding(encoding),
        data,
    })
}

fn language_position_encoding(encoding: &PositionEncodingKind) -> LanguagePositionEncoding {
    if *encoding == PositionEncodingKind::UTF8 {
        LanguagePositionEncoding::Utf8
    } else {
        LanguagePositionEncoding::Utf16
    }
}

fn symbol_kind(value: u32) -> Option<SymbolKind> {
    serde_json::from_value(Value::from(value)).ok()
}

fn protocol_location_range(range: LanguageLocationRange) -> Range {
    Range::new(
        Position::new(range.start.row, range.start.character),
        Position::new(range.end.row, range.end.character),
    )
}

fn file_uri(path: &std::path::Path) -> Option<Uri> {
    let url = url::Url::from_file_path(path).ok()?;
    url.as_str().parse().ok()
}

fn project_location_ranges(
    request_id: LanguageRequestId,
    kind: LanguageLocationKind,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    ranges: Vec<(Uri, Range, Range)>,
) -> LanguageLocations {
    let encoding = language_position_encoding(encoding);
    LanguageLocations {
        request_id,
        kind,
        source_path,
        source_revision,
        targets: ranges
            .into_iter()
            .filter_map(|(uri, range, selection_range)| {
                file_path(&uri).map(|path| LanguageLocationTarget {
                    path,
                    range: location_range(range),
                    selection_range: location_range(selection_range),
                    encoding,
                })
            })
            .collect(),
    }
}

fn location_range(range: Range) -> LanguageLocationRange {
    LanguageLocationRange {
        start: LanguageLocationPosition {
            row: range.start.line,
            character: range.start.character,
        },
        end: LanguageLocationPosition {
            row: range.end.line,
            character: range.end.character,
        },
    }
}

fn project_completion_item(
    item: CompletionItem,
    request_position: LanguageDocumentPosition,
    text: &str,
    encoding: &PositionEncodingKind,
) -> LanguageCompletionItem {
    let documentation = item.documentation.map(documentation_text);
    let insert_text = item.insert_text.unwrap_or_else(|| item.label.clone());
    let safe_format = item.insert_text_format != Some(InsertTextFormat::SNIPPET);
    let safe_side_effects = item
        .additional_text_edits
        .as_ref()
        .is_none_or(Vec::is_empty)
        && item.command.is_none();
    let edit = if safe_format && safe_side_effects {
        match item.text_edit {
            Some(edit) => completion_edit(edit, text, encoding),
            None => insertion_edit(request_position, text, &insert_text),
        }
    } else {
        None
    };
    LanguageCompletionItem {
        label: item.label,
        detail: item.detail,
        documentation,
        insert_text,
        edit,
    }
}

fn insertion_edit(
    position: LanguageDocumentPosition,
    text: &str,
    new_text: &str,
) -> Option<LanguageTextEdit> {
    let offset = byte_offset_for_position(
        text,
        Position::new(position.row, position.byte_offset),
        &PositionEncodingKind::UTF8,
    )?;
    Some(LanguageTextEdit {
        range: LanguageTextRange::new(offset..offset),
        new_text: new_text.into(),
    })
}

fn completion_edit(
    edit: CompletionTextEdit,
    text: &str,
    encoding: &PositionEncodingKind,
) -> Option<LanguageTextEdit> {
    let (range, new_text) = match edit {
        CompletionTextEdit::Edit(edit) => (edit.range, edit.new_text),
        CompletionTextEdit::InsertAndReplace(edit) => (edit.replace, edit.new_text),
    };
    Some(LanguageTextEdit {
        range: LanguageTextRange::new(byte_range_for_lsp_range(
            text,
            range.start,
            range.end,
            encoding,
        )?),
        new_text,
    })
}

fn hover_contents(contents: HoverContents) -> String {
    match contents {
        HoverContents::Scalar(value) => marked_string(value),
        HoverContents::Array(values) => values
            .into_iter()
            .map(marked_string)
            .collect::<Vec<_>>()
            .join("\n\n"),
        HoverContents::Markup(content) => content.value,
    }
}

fn marked_string(value: MarkedString) -> String {
    match value {
        MarkedString::String(value) => value,
        MarkedString::LanguageString(LanguageString { language, value }) => {
            format!("```{language}\n{value}\n```")
        }
    }
}

fn documentation_text(documentation: Documentation) -> String {
    match documentation {
        Documentation::String(value) => value,
        Documentation::MarkupContent(MarkupContent { value, .. }) => value,
    }
}

fn source_line(text: &str, requested: usize) -> Option<&str> {
    let mut lines = text.split('\n');
    let line = lines.nth(requested)?;
    Some(line.strip_suffix('\r').unwrap_or(line))
}

fn file_path(uri: &Uri) -> Option<PathBuf> {
    let url = url::Url::parse(&uri.to_string()).ok()?;
    url.to_file_path().ok()
}

#[cfg(test)]
#[path = "requests_tests.rs"]
mod tests;
