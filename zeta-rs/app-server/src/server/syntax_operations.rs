use super::{AppServer, ConnectionState, RpcError, decode, result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::syntax::{
    SyntaxAnalysisSnapshotDto, SyntaxChangeParams, SyntaxCloseParams, SyntaxLanguageDto,
    SyntaxOpenParams, SyntaxTextEditDto,
};
use zeta_syntax::{
    AnalysisLimits, DocumentRevision, SyntaxDocument, SyntaxEdit, SyntaxError, SyntaxLanguage,
};

mod snapshot_encoding;

use snapshot_encoding::syntax_analysis_snapshot;

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
    ) -> Result<SyntaxAnalysisSnapshotDto, SyntaxAnalysisError> {
        validate_document_id(&params.document_id)?;
        validate_document_uri(&params.document_uri)?;
        let language = match params.language {
            SyntaxLanguageDto::Json => SyntaxLanguage::Json,
            SyntaxLanguageDto::Jsonc => SyntaxLanguage::Jsonc,
            SyntaxLanguageDto::Rust => SyntaxLanguage::Rust,
        };
        let limits = AnalysisLimits {
            max_tokens: MAX_SYNTAX_TOKENS,
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
        let snapshot = syntax_analysis_snapshot(document.text(), &derived)?;
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
    ) -> Result<SyntaxAnalysisSnapshotDto, SyntaxAnalysisError> {
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
        syntax_analysis_snapshot(document.text(), &snapshot)
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

#[cfg(test)]
#[path = "syntax_operations_tests.rs"]
mod tests;
