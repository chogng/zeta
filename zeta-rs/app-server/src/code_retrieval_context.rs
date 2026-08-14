use std::num::NonZeroUsize;
use std::sync::Arc;

use zeta_async_utils::CancellationToken;
use zeta_code_index::CodeIndex;
use zeta_code_index_cloud::CloudCodeIndexController;
use zeta_code_index_semantic::CodeIndexSemanticService;
use zeta_code_retrieval::CodeRetrievalBudget;
use zeta_code_retrieval::CodeRetrievalOrigin;
use zeta_code_retrieval::CodeRetrievalQuery;
use zeta_code_retrieval::CodeRetrievalService;
use zeta_config::ConfigStore;
use zeta_config::SemanticCodeIndexAutomaticContext;
use zeta_core::ContextEvidence;
use zeta_core::ContextSource;
use zeta_core::ContextSourceRequest;
use zeta_core::CoreError;
use zeta_symbol_index::SymbolIndex;
use zeta_workspace::WorkspaceTrustId;

const MAX_EVIDENCE_ITEMS: usize = 8;
const MAX_EVIDENCE_ITEM_BYTES: usize = 12 * 1024;
const MAX_EVIDENCE_TOTAL_BYTES: usize = 48 * 1024;

/// Adapts current-source-verified code retrieval into Core's generic evidence contract.
pub(crate) struct CodeRetrievalContextSource {
    index: Arc<CodeIndex>,
    symbol_index: Option<Arc<SymbolIndex>>,
    semantic: Option<Arc<CodeIndexSemanticService>>,
    cloud: Option<Arc<CloudCodeIndexController>>,
    config: Option<Arc<ConfigStore>>,
    workspace: WorkspaceTrustId,
}

impl CodeRetrievalContextSource {
    pub(crate) fn new(
        index: Arc<CodeIndex>,
        symbol_index: Option<Arc<SymbolIndex>>,
        semantic: Option<Arc<CodeIndexSemanticService>>,
        cloud: Option<Arc<CloudCodeIndexController>>,
        config: Option<Arc<ConfigStore>>,
        workspace: WorkspaceTrustId,
    ) -> Self {
        Self {
            index,
            symbol_index,
            semantic,
            cloud,
            config,
            workspace,
        }
    }

    fn enabled(&self) -> bool {
        self.config
            .as_ref()
            .and_then(|config| config.read_snapshot().ok())
            .is_some_and(|snapshot| {
                snapshot.values.semantic_code_index.automatic_context
                    == SemanticCodeIndexAutomaticContext::FirstInvocation
                    && snapshot
                        .values
                        .semantic_code_index
                        .authorized_remote_models(&self.workspace, &snapshot.values.providers)
                        .is_some()
            })
    }

    fn service(&self) -> Result<CodeRetrievalService, CoreError> {
        let service = match (&self.semantic, &self.cloud) {
            (Some(semantic), Some(cloud)) => CodeRetrievalService::local_semantic_with_cloud(
                Arc::clone(&self.index),
                Arc::clone(semantic),
                Arc::clone(cloud),
            ),
            (Some(semantic), None) => {
                CodeRetrievalService::local_semantic(Arc::clone(&self.index), Arc::clone(semantic))
            }
            (None, Some(cloud)) => {
                CodeRetrievalService::hybrid(Arc::clone(&self.index), Arc::clone(cloud))
            }
            (None, None) => Ok(CodeRetrievalService::local(Arc::clone(&self.index))),
        }
        .map_err(|error| CoreError::Context(error.to_string()))?;
        let service = match &self.symbol_index {
            Some(symbol_index) => service
                .with_symbol_index(Arc::clone(symbol_index))
                .map_err(|error| CoreError::Context(error.to_string()))?,
            None => service,
        };
        Ok(service.with_budget(
            CodeRetrievalBudget::default()
                .with_max_item_bytes(NonZeroUsize::new(MAX_EVIDENCE_ITEM_BYTES).unwrap())
                .with_max_total_bytes(NonZeroUsize::new(MAX_EVIDENCE_TOTAL_BYTES).unwrap()),
        ))
    }
}

impl ContextSource for CodeRetrievalContextSource {
    fn collect(
        &self,
        request: &ContextSourceRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ContextEvidence>, CoreError> {
        if !self.enabled() {
            return Ok(Vec::new());
        }
        let query = CodeRetrievalQuery::new(
            request.query,
            NonZeroUsize::new(MAX_EVIDENCE_ITEMS).unwrap(),
        )
        .map_err(|error| CoreError::Context(error.to_string()))?;
        let retrieval = self
            .service()?
            .retrieve_with_cancellation(&query, cancellation)
            .map_err(|error| match error {
                zeta_code_retrieval::CodeRetrievalError::Cancelled(message) => {
                    CoreError::Cancelled(message)
                }
                error => CoreError::Context(error.to_string()),
            })?;
        Ok(retrieval
            .hits
            .into_iter()
            .map(|hit| {
                let source = if hit.origins.contains(&CodeRetrievalOrigin::LocalSymbol) {
                    "code-index/local-symbol"
                } else {
                    "code-index"
                };
                ContextEvidence {
                    source: source.into(),
                    reference: format!(
                        "{}:{}-{}",
                        hit.reference.relative_path.display(),
                        hit.reference.span.start_line.saturating_add(1),
                        hit.reference.span.end_line_exclusive
                    ),
                    revision: hit.reference.source_revision.as_str().to_owned(),
                    body: hit.content,
                }
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "code_retrieval_context_tests.rs"]
mod tests;
