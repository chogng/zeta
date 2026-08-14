use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use serde_json::Value;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::language::LanguagePositionDto;
use zeta_app_server_protocol::protocol::language::LanguageRangeDto;
use zeta_app_server_protocol::protocol::symbol_index::SymbolIndexSearchHitDto;
use zeta_app_server_protocol::protocol::symbol_index::SymbolIndexSearchParams;
use zeta_app_server_protocol::protocol::symbol_index::SymbolIndexSearchResult;
use zeta_app_server_protocol::protocol::symbol_index::SymbolIndexStateDto;
use zeta_app_server_protocol::protocol::symbol_index::SymbolIndexStatusResult;
use zeta_app_server_protocol::protocol::symbol_index::SymbolKindDto;
use zeta_app_server_protocol::protocol::symbol_index::WorkspaceDocumentOverlayCloseParams;
use zeta_app_server_protocol::protocol::symbol_index::WorkspaceDocumentOverlayStatusResult;
use zeta_app_server_protocol::protocol::symbol_index::WorkspaceDocumentOverlaySynchronizeParams;
use zeta_code_index::CodeIndexOverlayDocument;
use zeta_code_index::IndexedLanguage;
use zeta_code_index::IndexedSourceReference;
use zeta_symbol_index::SymbolIndexError;
use zeta_symbol_index::SymbolIndexQuery;
use zeta_symbol_index::SymbolIndexSnapshot;
use zeta_symbol_index::SymbolKind;
use zeta_symbol_index::SymbolRange;
use zeta_symbol_index::SymbolSearchHit;

use super::AppServer;
use super::RpcError;
use super::decode;
use super::result;
use super::symbol_index_runtime::SymbolIndexRuntime;
use super::symbol_index_runtime::SymbolIndexRuntimeError;
use super::symbol_index_runtime::SymbolIndexRuntimeState;

const MAX_PROTOCOL_RESULTS: usize = 100;

impl AppServer {
    pub(super) fn symbol_index_status(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let runtime = self.symbol_index_service()?;
        result(&project_status(&runtime))
    }

    pub(super) fn symbol_index_search(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SymbolIndexSearchParams = decode(params)?;
        let result_limit = NonZeroUsize::new(params.max_results)
            .filter(|value| value.get() <= MAX_PROTOCOL_RESULTS)
            .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let query = SymbolIndexQuery::new(params.query).with_result_limit(result_limit);
        let runtime = self.symbol_index_service()?;
        let code_index = self.code_index_service()?;
        let hits = runtime.search(&query).map_err(symbol_index_runtime_error)?;
        let (hits, discarded_stale_hit_count) = project_verified_hits(code_index.index(), hits);
        result(&SymbolIndexSearchResult {
            status: project_status(&runtime),
            hits,
            discarded_stale_hit_count,
        })
    }

    pub(super) fn workspace_document_overlay_synchronize(
        &self,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceDocumentOverlaySynchronizeParams = decode(params)?;
        let code_index = self.code_index_service()?;
        let symbol_index = self.symbol_index_service()?;
        let snapshot = code_index
            .index()
            .synchronize_overlay(CodeIndexOverlayDocument {
                relative_path: params.document.path,
                editor_revision: params.document.revision,
                language: indexed_language(&params.document.language_id),
                content: params.document.text,
            })
            .map_err(code_index_overlay_error)?;
        symbol_index
            .reconcile_overlay()
            .map_err(symbol_index_error)?;
        result(&WorkspaceDocumentOverlayStatusResult {
            generation: snapshot.generation,
            dirty_document_count: snapshot.documents.len(),
        })
    }

    pub(super) fn workspace_document_overlay_close(
        &self,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceDocumentOverlayCloseParams = decode(params)?;
        let code_index = self.code_index_service()?;
        let symbol_index = self.symbol_index_service()?;
        let snapshot = code_index
            .index()
            .close_overlay(&params.path)
            .map_err(code_index_overlay_error)?;
        symbol_index
            .reconcile_overlay()
            .map_err(symbol_index_error)?;
        result(&WorkspaceDocumentOverlayStatusResult {
            generation: snapshot.generation,
            dirty_document_count: snapshot.documents.len(),
        })
    }
}

pub(super) fn project_status(runtime: &SymbolIndexRuntime) -> SymbolIndexStatusResult {
    let (state, snapshot) = match runtime.state() {
        SymbolIndexRuntimeState::Empty => (SymbolIndexStateDto::Empty, None),
        SymbolIndexRuntimeState::Indexing { last_ready } => {
            (SymbolIndexStateDto::Indexing, last_ready)
        }
        SymbolIndexRuntimeState::Ready(snapshot) => (SymbolIndexStateDto::Ready, Some(snapshot)),
        SymbolIndexRuntimeState::Stale(snapshot) => (SymbolIndexStateDto::Stale, Some(snapshot)),
        SymbolIndexRuntimeState::Failed => (SymbolIndexStateDto::Failed, None),
    };
    project_snapshot(runtime, state, snapshot.as_ref())
}

fn project_snapshot(
    runtime: &SymbolIndexRuntime,
    state: SymbolIndexStateDto,
    snapshot: Option<&SymbolIndexSnapshot>,
) -> SymbolIndexStatusResult {
    SymbolIndexStatusResult {
        state,
        root_id: runtime.root_id().as_str().to_owned(),
        generation: snapshot.map_or(0, |snapshot| snapshot.generation),
        source_generation: snapshot.map_or(0, |snapshot| snapshot.source_generation),
        indexed_source_count: snapshot.map_or(0, |snapshot| snapshot.indexed_source_count),
        indexed_symbol_count: snapshot.map_or(0, |snapshot| snapshot.indexed_symbol_count),
        symbol_limit_hit: snapshot.is_some_and(|snapshot| snapshot.symbol_limit_hit),
    }
}

fn project_verified_hits(
    code_index: std::sync::Arc<zeta_code_index::CodeIndex>,
    hits: Vec<SymbolSearchHit>,
) -> (Vec<SymbolIndexSearchHitDto>, usize) {
    let mut sources = BTreeMap::<(PathBuf, String), Option<String>>::new();
    let mut projected = Vec::with_capacity(hits.len());
    let mut discarded = 0;
    for hit in hits {
        let reference = &hit.symbol.reference;
        let key = (
            reference.relative_path.clone(),
            reference.source_revision.as_str().to_owned(),
        );
        let content = sources.entry(key).or_insert_with(|| {
            code_index
                .materialize_sources(&[IndexedSourceReference {
                    root_id: reference.root_id.clone(),
                    relative_path: reference.relative_path.clone(),
                    source_revision: reference.source_revision.clone(),
                    language: reference.language,
                    source_bytes: reference.source_bytes,
                }])
                .ok()
                .and_then(|mut sources| sources.pop())
                .map(|source| source.content)
        });
        let Some(content) = content else {
            discarded += 1;
            continue;
        };
        let Some(declaration_range) = project_range(content, &reference.declaration_range) else {
            discarded += 1;
            continue;
        };
        let Some(selection_range) = project_range(content, &reference.selection_range) else {
            discarded += 1;
            continue;
        };
        projected.push(SymbolIndexSearchHitDto {
            name: hit.symbol.name.clone(),
            kind: project_kind(hit.symbol.kind),
            container_name: hit.symbol.container_name,
            path: reference.relative_path.clone(),
            language: reference.language.id().to_owned(),
            source_revision: reference.source_revision.as_str().to_owned(),
            declaration_range,
            selection_range,
            score: hit.score,
            matched_indices: project_matched_indices(&hit.symbol.name, &hit.matched_indices),
        });
    }
    (projected, discarded)
}

fn project_range(text: &str, range: &SymbolRange) -> Option<LanguageRangeDto> {
    Some(LanguageRangeDto {
        start: project_position(text, range.start_byte, range.start_line)?,
        end: project_position(text, range.end_byte, range.end_line)?,
    })
}

fn project_position(
    text: &str,
    byte_offset: usize,
    expected_line: usize,
) -> Option<LanguagePositionDto> {
    let prefix = text.get(..byte_offset)?;
    let line_index = prefix.bytes().filter(|byte| *byte == b'\n').count();
    if line_index != expected_line {
        return None;
    }
    let line = prefix.rsplit_once('\n').map_or(prefix, |(_, line)| line);
    Some(LanguagePositionDto {
        line_index: u32::try_from(line_index).ok()?,
        column_index: u32::try_from(line.encode_utf16().count()).ok()?,
    })
}

fn project_matched_indices(name: &str, indices: &[u32]) -> Vec<u32> {
    let requested = indices
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut utf16_offset = 0u32;
    name.chars()
        .enumerate()
        .filter_map(|(scalar_offset, character)| {
            let projected = requested
                .contains(&u32::try_from(scalar_offset).ok()?)
                .then_some(utf16_offset);
            utf16_offset = utf16_offset.saturating_add(character.len_utf16() as u32);
            projected
        })
        .collect()
}

fn project_kind(kind: SymbolKind) -> SymbolKindDto {
    match kind {
        SymbolKind::Constant => SymbolKindDto::Constant,
        SymbolKind::Enum => SymbolKindDto::Enum,
        SymbolKind::Field => SymbolKindDto::Field,
        SymbolKind::Function => SymbolKindDto::Function,
        SymbolKind::Macro => SymbolKindDto::Macro,
        SymbolKind::Method => SymbolKindDto::Method,
        SymbolKind::Module => SymbolKindDto::Module,
        SymbolKind::Static => SymbolKindDto::Static,
        SymbolKind::Struct => SymbolKindDto::Struct,
        SymbolKind::Trait => SymbolKindDto::Trait,
        SymbolKind::Type => SymbolKindDto::Type,
        SymbolKind::Variable => SymbolKindDto::Variable,
    }
}

fn indexed_language(language_id: &str) -> IndexedLanguage {
    match language_id {
        "javascript" => IndexedLanguage::Javascript,
        "javascriptreact" => IndexedLanguage::JavascriptReact,
        "json" => IndexedLanguage::Json,
        "jsonc" => IndexedLanguage::Jsonc,
        "rust" => IndexedLanguage::Rust,
        "shell" => IndexedLanguage::Shell,
        "typescript" => IndexedLanguage::TypeScript,
        "typescriptreact" => IndexedLanguage::TypeScriptReact,
        _ => IndexedLanguage::PlainText,
    }
}

fn code_index_overlay_error(error: zeta_code_index::CodeIndexError) -> RpcError {
    match error {
        zeta_code_index::CodeIndexError::InvalidOverlayPath
        | zeta_code_index::CodeIndexError::OverlayRevisionConflict
        | zeta_code_index::CodeIndexError::SourceVerificationLimitExceeded => {
            RpcError::new(-32602, AppServerErrorName::InvalidParams)
        }
        _ => RpcError::new(-32094, AppServerErrorName::SymbolIndexOperationFailed),
    }
}

fn symbol_index_runtime_error(error: SymbolIndexRuntimeError) -> RpcError {
    match error {
        SymbolIndexRuntimeError::NotReady => {
            RpcError::new(-32093, AppServerErrorName::SymbolIndexNotReady)
        }
        SymbolIndexRuntimeError::SourceIndex(error) => {
            log::warn!("symbol-index source generation check failed: {error}");
            RpcError::new(-32094, AppServerErrorName::SymbolIndexOperationFailed)
        }
        SymbolIndexRuntimeError::Index(error) => symbol_index_error(error),
    }
}

fn symbol_index_error(error: SymbolIndexError) -> RpcError {
    match error {
        SymbolIndexError::QueryTooLarge => RpcError::new(-32602, AppServerErrorName::InvalidParams),
        _ => RpcError::new(-32094, AppServerErrorName::SymbolIndexOperationFailed),
    }
}

#[cfg(test)]
#[path = "symbol_index_operations_tests.rs"]
mod tests;
