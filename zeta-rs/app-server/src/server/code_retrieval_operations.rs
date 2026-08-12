use std::num::NonZeroUsize;

use serde_json::Value;
use zeta_app_server_protocol::protocol::code_index::CodeIndexChunkSpanDto;
use zeta_app_server_protocol::protocol::code_index::CodeRetrievalDegradationDto;
use zeta_app_server_protocol::protocol::code_index::CodeRetrievalHitDto;
use zeta_app_server_protocol::protocol::code_index::CodeRetrievalOriginDto;
use zeta_app_server_protocol::protocol::code_index::CodeRetrievalParams;
use zeta_app_server_protocol::protocol::code_index::CodeRetrievalResult;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_code_index_cloud::CodeIndexDeploymentMode;
use zeta_code_retrieval::CodeRetrievalDegradation;
use zeta_code_retrieval::CodeRetrievalError;
use zeta_code_retrieval::CodeRetrievalHit;
use zeta_code_retrieval::CodeRetrievalOrigin;
use zeta_code_retrieval::CodeRetrievalQuery;
use zeta_code_retrieval::CodeRetrievalService;

use super::AppServer;
use super::RpcError;
use super::code_index_operations::project_status;
use super::code_index_runtime::CodeIndexRuntimeError;
use super::decode;
use super::result;

const MAX_PROTOCOL_RESULTS: usize = 100;

impl AppServer {
    pub(super) fn code_retrieve(&self, params: &Value) -> Result<Value, RpcError> {
        let params: CodeRetrievalParams = decode(params)?;
        let result_limit = NonZeroUsize::new(params.max_results)
            .filter(|value| value.get() <= MAX_PROTOCOL_RESULTS)
            .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let query =
            CodeRetrievalQuery::new(params.query, result_limit).map_err(code_retrieval_error)?;
        let runtime = self.code_index_service()?;
        runtime
            .ensure_searchable()
            .map_err(code_index_runtime_error)?;
        let index = runtime.index();
        let service = match self.cloud_code_index_service() {
            Ok(cloud) => match cloud.status() {
                Ok(status) if status.deployment_mode == CodeIndexDeploymentMode::LocalOnly => {
                    CodeRetrievalService::local(index)
                }
                Ok(_) | Err(_) => {
                    CodeRetrievalService::hybrid(index, cloud).map_err(code_retrieval_error)?
                }
            },
            Err(_) => CodeRetrievalService::local(index),
        };
        let retrieval = service.retrieve(&query).map_err(code_retrieval_error)?;
        result(&CodeRetrievalResult {
            status: project_status(&runtime),
            hits: retrieval.hits.into_iter().map(project_hit).collect(),
            degradations: retrieval
                .degradations
                .into_iter()
                .map(project_degradation)
                .collect(),
        })
    }
}

fn project_hit(hit: CodeRetrievalHit) -> CodeRetrievalHitDto {
    CodeRetrievalHitDto {
        path: hit.reference.relative_path,
        language: hit.language.id().to_owned(),
        source_revision: hit.reference.source_revision.as_str().to_owned(),
        content_hash: hit.reference.content_hash.as_str().to_owned(),
        span: CodeIndexChunkSpanDto {
            start_byte: hit.reference.span.start_byte,
            end_byte: hit.reference.span.end_byte,
            start_line: hit.reference.span.start_line,
            end_line_exclusive: hit.reference.span.end_line_exclusive,
        },
        content: hit.content,
        rrf_score: hit.rrf_score,
        origins: hit.origins.into_iter().map(project_origin).collect(),
    }
}

fn project_origin(origin: CodeRetrievalOrigin) -> CodeRetrievalOriginDto {
    match origin {
        CodeRetrievalOrigin::LocalLexical => CodeRetrievalOriginDto::LocalLexical,
        CodeRetrievalOrigin::CloudSemantic => CodeRetrievalOriginDto::CloudSemantic,
    }
}

fn project_degradation(degradation: CodeRetrievalDegradation) -> CodeRetrievalDegradationDto {
    match degradation {
        CodeRetrievalDegradation::CloudQueryFailed => CodeRetrievalDegradationDto::CloudQueryFailed,
        CodeRetrievalDegradation::CandidateVerificationFailed { discarded } => {
            CodeRetrievalDegradationDto::CandidateVerificationFailed { discarded }
        }
        CodeRetrievalDegradation::ContentBudgetExceeded { discarded } => {
            CodeRetrievalDegradationDto::ContentBudgetExceeded { discarded }
        }
    }
}

fn code_index_runtime_error(error: CodeIndexRuntimeError) -> RpcError {
    match error {
        CodeIndexRuntimeError::NotReady => {
            RpcError::new(-32091, AppServerErrorName::CodeIndexNotReady)
        }
        CodeIndexRuntimeError::Index(_) => {
            RpcError::new(-32096, AppServerErrorName::CodeRetrievalOperationFailed)
        }
    }
}

fn code_retrieval_error(error: CodeRetrievalError) -> RpcError {
    match error {
        CodeRetrievalError::InvalidQuery(_) => {
            RpcError::new(-32602, AppServerErrorName::InvalidParams)
        }
        CodeRetrievalError::RootMismatch | CodeRetrievalError::LocalIndex(_) => {
            RpcError::new(-32096, AppServerErrorName::CodeRetrievalOperationFailed)
        }
    }
}
