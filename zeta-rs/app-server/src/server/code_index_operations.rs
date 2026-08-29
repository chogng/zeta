use super::AppServer;
use super::RpcError;
use super::code_index_runtime::CodeIndexRuntime;
use super::code_index_runtime::CodeIndexRuntimeError;
use super::code_index_runtime::CodeIndexRuntimeState;
use super::decode;
use super::result;
use serde_json::Value;
use std::num::NonZeroUsize;
use zeta_app_server_protocol::protocol::code_index::CodeIndexChunkSpanDto;
use zeta_app_server_protocol::protocol::code_index::CodeIndexSearchHitDto;
use zeta_app_server_protocol::protocol::code_index::CodeIndexSearchParams;
use zeta_app_server_protocol::protocol::code_index::CodeIndexSearchResult;
use zeta_app_server_protocol::protocol::code_index::CodeIndexStateDto;
use zeta_app_server_protocol::protocol::code_index::CodeIndexStatusResult;
use zeta_app_server_protocol::protocol::code_index::FastRegexIndexStatusResult;
use zeta_app_server_protocol::protocol::code_index::SemanticCodeIndexStateDto;
use zeta_app_server_protocol::protocol::code_index::SemanticCodeIndexStatusDto;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_code_index::CodeIndexError;
use zeta_code_index::CodeIndexQuery;
use zeta_code_index::CodeIndexSnapshot;
use zeta_code_index::SearchHit;

const MAX_PROTOCOL_RESULTS: usize = 100;

impl AppServer {
    pub(super) fn code_index_status(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let runtime = self.code_index_service()?;
        result(&self.project_code_index_status(&runtime))
    }

    pub(super) fn code_index_search(&self, params: &Value) -> Result<Value, RpcError> {
        let params: CodeIndexSearchParams = decode(params)?;
        let result_limit = NonZeroUsize::new(params.max_results)
            .filter(|value| value.get() <= MAX_PROTOCOL_RESULTS)
            .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let query = CodeIndexQuery::new(params.query).with_result_limit(result_limit);
        let runtime = self.code_index_service()?;
        let hits = runtime
            .search(&query)
            .map_err(code_index_runtime_error)?
            .into_iter()
            .map(project_hit)
            .collect();
        result(&CodeIndexSearchResult {
            status: self.project_code_index_status(&runtime),
            hits,
        })
    }

    pub(super) fn code_index_rebuild(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let runtime = self.code_index_service()?;
        runtime.rebuild().map_err(code_index_error)?;
        if let Err(error) = runtime.index().handoff_matching_overlays() {
            log::warn!("code-index overlay handoff failed after explicit rebuild: {error}");
        }
        if let Ok(symbol_index) = self.symbol_index_service()
            && let Err(error) = symbol_index.reconcile()
        {
            log::warn!("symbol-index reconcile failed after explicit code-index rebuild: {error}");
        }
        if let Ok(symbol_index) = self.symbol_index_service()
            && let Err(error) = symbol_index.reconcile_overlay()
        {
            log::warn!(
                "symbol-index overlay reconcile failed after explicit code-index rebuild: {error}"
            );
        }
        if let Some(job) = self.code_index_semantic_job() {
            job.schedule();
        }
        result(&self.project_code_index_status(&runtime))
    }

    pub(super) fn fast_regex_index_status(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let (service, root) = self.agent_grep_index_context()?;
        let snapshot = service
            .fast_regex_snapshot(&root)
            .map_err(|_| RpcError::new(-32092, AppServerErrorName::CodeIndexOperationFailed))?;
        result(&project_fast_regex_status(
            service.watches_fast_regex(),
            snapshot,
        ))
    }

    pub(super) fn fast_regex_index_rebuild(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let (service, root) = self.agent_grep_index_context()?;
        let snapshot = service
            .rebuild_fast_regex(&root)
            .map_err(|_| RpcError::new(-32092, AppServerErrorName::CodeIndexOperationFailed))?;
        result(&project_fast_regex_status(true, Some(snapshot)))
    }

    pub(super) fn semantic_code_index_cancel(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        if let Some(job) = self.code_index_semantic_job() {
            job.cancel();
        }
        let runtime = self.code_index_service()?;
        result(&self.project_code_index_status(&runtime))
    }

    pub(super) fn semantic_code_index_retry(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        if let Some(job) = self.code_index_semantic_job() {
            job.schedule();
        }
        let runtime = self.code_index_service()?;
        result(&self.project_code_index_status(&runtime))
    }

    pub(super) fn project_code_index_status(
        &self,
        runtime: &CodeIndexRuntime,
    ) -> CodeIndexStatusResult {
        let mut status = project_status(runtime);
        status.semantic =
            self.code_index_semantic_job()
                .map_or_else(unavailable_semantic_status, |job| {
                    let snapshot = job.snapshot();
                    SemanticCodeIndexStatusDto {
                        state: match snapshot.state {
                            super::semantic_index_job::SemanticIndexJobState::Idle => {
                                SemanticCodeIndexStateDto::Idle
                            }
                            super::semantic_index_job::SemanticIndexJobState::Syncing => {
                                SemanticCodeIndexStateDto::Syncing
                            }
                            super::semantic_index_job::SemanticIndexJobState::Ready => {
                                SemanticCodeIndexStateDto::Ready
                            }
                            super::semantic_index_job::SemanticIndexJobState::Stale => {
                                SemanticCodeIndexStateDto::Stale
                            }
                            super::semantic_index_job::SemanticIndexJobState::Cancelled => {
                                SemanticCodeIndexStateDto::Cancelled
                            }
                            super::semantic_index_job::SemanticIndexJobState::Failed => {
                                SemanticCodeIndexStateDto::Failed
                            }
                        },
                        operation_id: snapshot.operation_id,
                        target_generation: snapshot.target_generation,
                        published_generation: snapshot.published_generation,
                        phase: snapshot
                            .phase
                            .map(|phase| format!("{phase:?}").to_lowercase()),
                        total_chunk_count: snapshot.total_chunk_count,
                        processed_chunk_count: snapshot.processed_chunk_count,
                        reused_embedding_count: snapshot.reused_embedding_count,
                        embedded_chunk_count: snapshot.embedded_chunk_count,
                        completed_batch_count: snapshot.completed_batch_count,
                        total_batch_count: snapshot.total_batch_count,
                        retry_count: snapshot.retry_count,
                        last_error_code: snapshot.last_error_code.map(str::to_owned),
                    }
                });
        status
    }
}

fn project_fast_regex_status(
    enabled: bool,
    snapshot: Option<zeta_fast_regex_search::FastRegexSearchSnapshot>,
) -> FastRegexIndexStatusResult {
    FastRegexIndexStatusResult {
        enabled,
        active: snapshot.is_some(),
        generation: snapshot.as_ref().map(|snapshot| snapshot.generation),
        indexed_file_count: snapshot
            .as_ref()
            .map_or(0, |snapshot| snapshot.indexed_file_count),
        indexed_source_bytes: snapshot
            .as_ref()
            .map_or(0, |snapshot| snapshot.indexed_source_bytes),
    }
}

pub(super) fn project_status(runtime: &CodeIndexRuntime) -> CodeIndexStatusResult {
    let (state, snapshot) = match runtime.state() {
        CodeIndexRuntimeState::Empty => (CodeIndexStateDto::Empty, None),
        CodeIndexRuntimeState::Indexing { last_ready } => (CodeIndexStateDto::Indexing, last_ready),
        CodeIndexRuntimeState::Ready(snapshot) => (CodeIndexStateDto::Ready, Some(snapshot)),
        CodeIndexRuntimeState::Stale(snapshot) => (CodeIndexStateDto::Stale, Some(snapshot)),
        CodeIndexRuntimeState::Failed => (CodeIndexStateDto::Failed, None),
    };
    project_snapshot(runtime, state, snapshot.as_ref())
}

fn project_snapshot(
    runtime: &CodeIndexRuntime,
    state: CodeIndexStateDto,
    snapshot: Option<&CodeIndexSnapshot>,
) -> CodeIndexStatusResult {
    CodeIndexStatusResult {
        state,
        root_id: runtime.root().trust_id().as_str().to_owned(),
        generation: snapshot.map_or(0, |snapshot| snapshot.generation),
        indexed_file_count: snapshot.map_or(0, |snapshot| snapshot.indexed_file_count),
        indexed_chunk_count: snapshot.map_or(0, |snapshot| snapshot.indexed_chunk_count),
        indexed_source_bytes: snapshot.map_or(0, |snapshot| snapshot.indexed_source_bytes),
        skipped_file_count: snapshot.map_or(0, |snapshot| snapshot.skipped_file_count),
        truncated_file_count: snapshot.map_or(0, |snapshot| snapshot.truncated_file_count),
        file_limit_hit: snapshot.is_some_and(|snapshot| snapshot.file_limit_hit),
        source_bytes_limit_hit: snapshot.is_some_and(|snapshot| snapshot.source_bytes_limit_hit),
        semantic: unavailable_semantic_status(),
    }
}

fn unavailable_semantic_status() -> SemanticCodeIndexStatusDto {
    SemanticCodeIndexStatusDto {
        state: SemanticCodeIndexStateDto::Unavailable,
        operation_id: None,
        target_generation: 0,
        published_generation: None,
        phase: None,
        total_chunk_count: 0,
        processed_chunk_count: 0,
        reused_embedding_count: 0,
        embedded_chunk_count: 0,
        completed_batch_count: 0,
        total_batch_count: 0,
        retry_count: 0,
        last_error_code: None,
    }
}

fn project_hit(hit: SearchHit) -> CodeIndexSearchHitDto {
    CodeIndexSearchHitDto {
        path: hit.reference.relative_path,
        language: hit.language.id().to_owned(),
        source_revision: hit.reference.source_revision.as_str().to_owned(),
        chunk_key: hit.reference.key.as_str().to_owned(),
        content_hash: hit.reference.content_hash.as_str().to_owned(),
        span: CodeIndexChunkSpanDto {
            start_byte: hit.reference.span.start_byte,
            end_byte: hit.reference.span.end_byte,
            start_line: hit.reference.span.start_line,
            end_line_exclusive: hit.reference.span.end_line_exclusive,
        },
        content: hit.content,
        score: hit.score,
    }
}

fn code_index_runtime_error(error: CodeIndexRuntimeError) -> RpcError {
    match error {
        CodeIndexRuntimeError::NotReady => {
            RpcError::new(-32091, AppServerErrorName::CodeIndexNotReady)
        }
        CodeIndexRuntimeError::Index(error) => code_index_error(error),
    }
}

fn code_index_error(error: CodeIndexError) -> RpcError {
    match error {
        CodeIndexError::InvalidQuery(_) => RpcError::new(-32602, AppServerErrorName::InvalidParams),
        _ => RpcError::new(-32092, AppServerErrorName::CodeIndexOperationFailed),
    }
}

#[cfg(test)]
#[path = "code_index_operations_tests.rs"]
mod tests;
