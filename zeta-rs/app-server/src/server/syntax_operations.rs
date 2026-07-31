use super::{AppServer, ConnectionState, RpcError, decode, result};
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Mutex;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::syntax::{
    SyntaxChangeParams, SyntaxCloseParams, SyntaxLanguageDto, SyntaxOpenParams, SyntaxTextEditDto,
    SyntaxTokenSnapshotDto,
};
use zeta_syntax::{
    AnalysisLimits, DocumentRevision, SyntaxDocument, SyntaxEdit, SyntaxError, SyntaxLanguage,
    SyntaxSnapshot, SyntaxTokenKind,
};

const MAX_SYNTAX_TOKENS: usize = 50_000;

pub(super) struct SyntaxAnalysisService {
    documents: Mutex<BTreeMap<(u64, String), SyntaxDocument>>,
}

impl SyntaxAnalysisService {
    pub(super) fn new() -> Self {
        Self {
            documents: Mutex::new(BTreeMap::new()),
        }
    }

    fn open(
        &self,
        owner: u64,
        params: SyntaxOpenParams,
    ) -> Result<SyntaxTokenSnapshotDto, SyntaxAnalysisError> {
        validate_document_id(&params.document_id)?;
        validate_document_uri(&params.document_uri)?;
        let language = match params.language {
            SyntaxLanguageDto::Rust => SyntaxLanguage::Rust,
        };
        let limits = AnalysisLimits {
            max_tokens: MAX_SYNTAX_TOKENS,
            max_folding_ranges: 0,
            max_symbols: 0,
            max_diagnostics: 0,
            ..AnalysisLimits::default()
        };
        let document = SyntaxDocument::open_with_limits(
            language,
            document_revision(params.revision)?,
            params.text,
            limits,
        )
        .map_err(syntax_engine_error)?;
        let derived = document.snapshot();
        let snapshot = syntax_token_snapshot(document.text(), &derived)?;
        self.documents
            .lock()
            .map_err(|_| SyntaxAnalysisError::Failed)?
            .insert((owner, params.document_id), document);
        Ok(snapshot)
    }

    fn change(
        &self,
        owner: u64,
        params: SyntaxChangeParams,
    ) -> Result<SyntaxTokenSnapshotDto, SyntaxAnalysisError> {
        validate_document_id(&params.document_id)?;
        if params.edits.is_empty() || params.edits.len() > 1024 {
            return Err(SyntaxAnalysisError::InvalidInput);
        }
        let mut documents = self
            .documents
            .lock()
            .map_err(|_| SyntaxAnalysisError::Failed)?;
        let document = documents
            .get_mut(&(owner, params.document_id))
            .ok_or(SyntaxAnalysisError::NotOpen)?;
        if document.revision() != document_revision(params.previous_revision)?
            || params.revision <= params.previous_revision
        {
            return Err(SyntaxAnalysisError::RevisionMismatch);
        }
        let edits = syntax_edits(document.text(), params.edits)?;
        let snapshot = document
            .apply_edits(document_revision(params.revision)?, &edits)
            .map_err(syntax_engine_error)?;
        syntax_token_snapshot(document.text(), &snapshot)
    }

    fn close(&self, owner: u64, document_id: &str) -> Result<(), SyntaxAnalysisError> {
        self.documents
            .lock()
            .map_err(|_| SyntaxAnalysisError::Failed)?
            .remove(&(owner, document_id.to_owned()));
        Ok(())
    }

    pub(super) fn release_owner(&self, owner: u64) {
        if let Ok(mut documents) = self.documents.lock() {
            documents.retain(|(document_owner, _), _| *document_owner != owner);
        }
    }
}

impl AppServer {
    pub(super) fn syntax_open(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SyntaxOpenParams = decode(params)?;
        result(
            &self
                .syntax
                .open(connection.connection_id, params)
                .map_err(syntax_error)?,
        )
    }

    pub(super) fn syntax_change(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SyntaxChangeParams = decode(params)?;
        result(
            &self
                .syntax
                .change(connection.connection_id, params)
                .map_err(syntax_error)?,
        )
    }

    pub(super) fn syntax_close(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SyntaxCloseParams = decode(params)?;
        self.syntax
            .close(connection.connection_id, &params.document_id)
            .map_err(syntax_error)?;
        result(&())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SyntaxAnalysisError {
    InvalidInput,
    NotOpen,
    RevisionMismatch,
    Failed,
}

fn syntax_error(error: SyntaxAnalysisError) -> RpcError {
    match error {
        SyntaxAnalysisError::InvalidInput => {
            RpcError::new(-32602, AppServerErrorName::InvalidParams)
        }
        SyntaxAnalysisError::NotOpen => {
            RpcError::new(-32070, AppServerErrorName::SyntaxDocumentNotOpen)
        }
        SyntaxAnalysisError::RevisionMismatch => {
            RpcError::new(-32071, AppServerErrorName::SyntaxRevisionMismatch)
        }
        SyntaxAnalysisError::Failed => {
            RpcError::new(-32072, AppServerErrorName::SyntaxAnalysisFailed)
        }
    }
}

fn syntax_engine_error(error: SyntaxError) -> SyntaxAnalysisError {
    match error {
        SyntaxError::Language { .. } | SyntaxError::Query { .. } | SyntaxError::ParseCancelled => {
            SyntaxAnalysisError::Failed
        }
        SyntaxError::NonIncreasingRevision { .. }
        | SyntaxError::InvalidEditRange { .. }
        | SyntaxError::InvalidEditBoundary { .. }
        | SyntaxError::OverlappingEdits
        | SyntaxError::DocumentTooLarge { .. } => SyntaxAnalysisError::InvalidInput,
    }
}

fn document_revision(value: usize) -> Result<DocumentRevision, SyntaxAnalysisError> {
    Ok(DocumentRevision::new(
        value
            .try_into()
            .map_err(|_| SyntaxAnalysisError::InvalidInput)?,
    ))
}

fn validate_document_uri(document_uri: &str) -> Result<(), SyntaxAnalysisError> {
    if document_uri.is_empty() || document_uri.len() > 16_384 || document_uri.contains('\0') {
        return Err(SyntaxAnalysisError::InvalidInput);
    }
    Ok(())
}

fn validate_document_id(document_id: &str) -> Result<(), SyntaxAnalysisError> {
    if document_id.is_empty() || document_id.len() > 256 || document_id.contains('\0') {
        return Err(SyntaxAnalysisError::InvalidInput);
    }
    Ok(())
}

fn syntax_edits(
    text: &str,
    edits: Vec<SyntaxTextEditDto>,
) -> Result<Vec<SyntaxEdit>, SyntaxAnalysisError> {
    let mut requested = edits
        .iter()
        .flat_map(|edit| [edit.start_utf16, edit.end_utf16])
        .collect::<Vec<_>>();
    if edits.iter().any(|edit| edit.start_utf16 > edit.end_utf16) {
        return Err(SyntaxAnalysisError::InvalidInput);
    }
    requested.sort_unstable();
    requested.dedup();
    let mut resolved = BTreeMap::new();
    let mut requested_index = 0;
    let mut utf16_offset = 0;
    for (byte_offset, character) in text.char_indices() {
        while requested.get(requested_index) == Some(&utf16_offset) {
            resolved.insert(utf16_offset, byte_offset);
            requested_index += 1;
        }
        utf16_offset += character.len_utf16();
        if requested
            .get(requested_index)
            .is_some_and(|requested| *requested < utf16_offset)
        {
            return Err(SyntaxAnalysisError::InvalidInput);
        }
    }
    while requested.get(requested_index) == Some(&utf16_offset) {
        resolved.insert(utf16_offset, text.len());
        requested_index += 1;
    }
    if requested_index != requested.len() {
        return Err(SyntaxAnalysisError::InvalidInput);
    }
    edits
        .into_iter()
        .map(|edit| {
            Ok(SyntaxEdit::replace(
                *resolved
                    .get(&edit.start_utf16)
                    .ok_or(SyntaxAnalysisError::InvalidInput)?
                    ..*resolved
                        .get(&edit.end_utf16)
                        .ok_or(SyntaxAnalysisError::InvalidInput)?,
                edit.text,
            ))
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TokenSegment {
    bytes: Range<usize>,
    kind: SyntaxTokenKind,
}

fn syntax_token_snapshot(
    text: &str,
    snapshot: &SyntaxSnapshot,
) -> Result<SyntaxTokenSnapshotDto, SyntaxAnalysisError> {
    Ok(SyntaxTokenSnapshotDto {
        revision: snapshot
            .revision()
            .value()
            .try_into()
            .map_err(|_| SyntaxAnalysisError::Failed)?,
        result_id: snapshot.revision().value().to_string(),
        data: encode_semantic_tokens(text, snapshot)?,
    })
}

fn encode_semantic_tokens(
    text: &str,
    snapshot: &SyntaxSnapshot,
) -> Result<Vec<u32>, SyntaxAnalysisError> {
    let line_starts = line_starts(text);
    let mut lines = BTreeMap::<usize, Vec<TokenSegment>>::new();
    for token in snapshot.tokens() {
        for line in token.range.start.row..=token.range.end.row {
            let Some(&line_start) = line_starts.get(line) else {
                return Err(SyntaxAnalysisError::Failed);
            };
            let line_end = line_content_end(text, &line_starts, line);
            let start = token.range.bytes.start.max(line_start);
            let end = token.range.bytes.end.min(line_end);
            if start < end {
                overlay_segment(
                    lines.entry(line).or_default(),
                    TokenSegment {
                        bytes: start..end,
                        kind: token.kind,
                    },
                );
            }
        }
    }

    let mut data = Vec::new();
    let mut previous_line = 0usize;
    let mut previous_start = 0usize;
    let mut first = true;
    for (line, mut segments) in lines {
        segments.sort_by_key(|segment| (segment.bytes.start, Reverse(segment.bytes.end)));
        for segment in merge_adjacent_segments(segments) {
            let line_start = line_starts[line];
            let start = text[line_start..segment.bytes.start].encode_utf16().count();
            let length = text[segment.bytes.clone()].encode_utf16().count();
            if length == 0 {
                continue;
            }
            let delta_line = if first { line } else { line - previous_line };
            let delta_start = if !first && delta_line == 0 {
                start - previous_start
            } else {
                start
            };
            data.extend([
                to_u32(delta_line)?,
                to_u32(delta_start)?,
                to_u32(length)?,
                token_type(segment.kind),
                0,
            ]);
            previous_line = line;
            previous_start = start;
            first = false;
        }
    }
    Ok(data)
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        text.bytes()
            .enumerate()
            .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
    );
    starts
}

fn line_content_end(text: &str, starts: &[usize], line: usize) -> usize {
    let mut end = starts.get(line + 1).copied().unwrap_or(text.len());
    if end > 0 && text.as_bytes()[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && text.as_bytes()[end - 1] == b'\r' {
        end -= 1;
    }
    end
}

fn overlay_segment(segments: &mut Vec<TokenSegment>, overlay: TokenSegment) {
    let mut next = Vec::with_capacity(segments.len() + 1);
    for segment in segments.drain(..) {
        if segment.bytes.end <= overlay.bytes.start || segment.bytes.start >= overlay.bytes.end {
            next.push(segment);
            continue;
        }
        if segment.bytes.start < overlay.bytes.start {
            next.push(TokenSegment {
                bytes: segment.bytes.start..overlay.bytes.start,
                kind: segment.kind,
            });
        }
        if segment.bytes.end > overlay.bytes.end {
            next.push(TokenSegment {
                bytes: overlay.bytes.end..segment.bytes.end,
                kind: segment.kind,
            });
        }
    }
    next.push(overlay);
    *segments = next;
}

fn merge_adjacent_segments(mut segments: Vec<TokenSegment>) -> Vec<TokenSegment> {
    let mut merged: Vec<TokenSegment> = Vec::with_capacity(segments.len());
    for segment in segments.drain(..) {
        if let Some(previous) = merged.last_mut()
            && previous.kind == segment.kind
            && previous.bytes.end == segment.bytes.start
        {
            previous.bytes.end = segment.bytes.end;
        } else {
            merged.push(segment);
        }
    }
    merged
}

fn token_type(kind: SyntaxTokenKind) -> u32 {
    match kind {
        SyntaxTokenKind::Attribute => 0,
        SyntaxTokenKind::Comment => 1,
        SyntaxTokenKind::Constant => 2,
        SyntaxTokenKind::Constructor => 3,
        SyntaxTokenKind::Embedded => 4,
        SyntaxTokenKind::Function => 5,
        SyntaxTokenKind::Keyword => 6,
        SyntaxTokenKind::Label => 7,
        SyntaxTokenKind::Module => 8,
        SyntaxTokenKind::Number => 9,
        SyntaxTokenKind::Operator => 10,
        SyntaxTokenKind::Property => 11,
        SyntaxTokenKind::Punctuation => 10,
        SyntaxTokenKind::String => 4,
        SyntaxTokenKind::Type => 12,
        SyntaxTokenKind::Variable => 13,
    }
}

fn to_u32(value: usize) -> Result<u32, SyntaxAnalysisError> {
    value.try_into().map_err(|_| SyntaxAnalysisError::Failed)
}

#[cfg(test)]
#[path = "syntax_operations_tests.rs"]
mod tests;
