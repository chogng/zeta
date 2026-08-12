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
        result(&project_status(&runtime))
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
            status: project_status(&runtime),
            hits,
        })
    }

    pub(super) fn code_index_rebuild(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        let runtime = self.code_index_service()?;
        runtime.rebuild().map_err(code_index_error)?;
        result(&project_status(&runtime))
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
