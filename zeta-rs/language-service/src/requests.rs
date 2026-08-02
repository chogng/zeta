//! Product-neutral language request inputs and projected results.

use std::path::PathBuf;

use zeta_lsp::lsp_types::{
    CompletionItem, CompletionResponse, CompletionTextEdit, Documentation, GotoDefinitionResponse,
    Hover, HoverContents, InsertTextFormat, LanguageString, MarkedString, MarkupContent, Position,
    PositionEncodingKind, Uri,
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
    Definition,
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

/// One filesystem target returned by a definition request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageDefinitionTarget {
    pub path: PathBuf,
    pub row: u32,
    pub character: u32,
    pub encoding: LanguagePositionEncoding,
}

/// Fresh definition targets bound to the exact requesting revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageDefinitions {
    pub request_id: LanguageRequestId,
    pub source_path: PathBuf,
    pub source_revision: LanguageDocumentRevision,
    pub targets: Vec<LanguageDefinitionTarget>,
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

pub(crate) fn project_definitions(
    request_id: LanguageRequestId,
    source_path: PathBuf,
    source_revision: LanguageDocumentRevision,
    encoding: &PositionEncodingKind,
    response: Option<GotoDefinitionResponse>,
) -> LanguageDefinitions {
    let positions = match response {
        Some(GotoDefinitionResponse::Scalar(location)) => {
            vec![(location.uri, location.range.start)]
        }
        Some(GotoDefinitionResponse::Array(locations)) => locations
            .into_iter()
            .map(|location| (location.uri, location.range.start))
            .collect(),
        Some(GotoDefinitionResponse::Link(links)) => links
            .into_iter()
            .map(|link| (link.target_uri, link.target_selection_range.start))
            .collect(),
        None => Vec::new(),
    };
    let encoding = if *encoding == PositionEncodingKind::UTF8 {
        LanguagePositionEncoding::Utf8
    } else {
        LanguagePositionEncoding::Utf16
    };
    LanguageDefinitions {
        request_id,
        source_path,
        source_revision,
        targets: positions
            .into_iter()
            .filter_map(|(uri, position)| {
                file_path(&uri).map(|path| LanguageDefinitionTarget {
                    path,
                    row: position.line,
                    character: position.character,
                    encoding,
                })
            })
            .collect(),
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
