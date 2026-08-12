use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use zeta_async_utils::CancellationSource;
use zeta_async_utils::CancellationToken;
use zeta_code_index::ChunkReference;
use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexQuery;
use zeta_code_index::SourceExcerptReference;
use zeta_code_index_cloud::CloudCodeIndexController;
use zeta_code_index_cloud::CloudCodeIndexQuery;
use zeta_code_index_semantic::CodeIndexSemanticQuery;
use zeta_code_index_semantic::CodeIndexSemanticService;

use crate::CodeRetrievalBudget;
use crate::CodeRetrievalDegradation;
use crate::CodeRetrievalError;
use crate::CodeRetrievalHit;
use crate::CodeRetrievalOrigin;
use crate::CodeRetrievalQuery;
use crate::CodeRetrievalResult;

const RRF_RANK_CONSTANT: f64 = 60.0;
const CANDIDATE_MULTIPLIER: usize = 4;
const MAX_CANDIDATES_PER_SOURCE: usize = 100;

enum RetrievalDeployment {
    LocalOnly,
    LocalSemantic(Arc<CodeIndexSemanticService>),
    Cloud(Arc<CloudCodeIndexController>),
    Hybrid {
        semantic: Arc<CodeIndexSemanticService>,
        cloud: Arc<CloudCodeIndexController>,
    },
}

/// Workspace-scoped local/cloud candidate coordinator.
pub struct CodeRetrievalService {
    index: Arc<CodeIndex>,
    deployment: RetrievalDeployment,
    budget: CodeRetrievalBudget,
}

impl CodeRetrievalService {
    /// Creates a service that never invokes a cloud provider.
    pub fn local(index: Arc<CodeIndex>) -> Self {
        Self {
            index,
            deployment: RetrievalDeployment::LocalOnly,
            budget: CodeRetrievalBudget::default(),
        }
    }

    /// Creates a service that fuses cloud candidates and falls back to local candidates on any
    /// non-fatal cloud query failure.
    pub fn hybrid(
        index: Arc<CodeIndex>,
        cloud: Arc<CloudCodeIndexController>,
    ) -> Result<Self, CodeRetrievalError> {
        if index.root_id() != cloud.root_id() {
            return Err(CodeRetrievalError::RootMismatch);
        }
        Ok(Self {
            index,
            deployment: RetrievalDeployment::Cloud(cloud),
            budget: CodeRetrievalBudget::default(),
        })
    }

    /// Creates a local-first service that fuses lexical and local semantic candidates.
    pub fn local_semantic(
        index: Arc<CodeIndex>,
        semantic: Arc<CodeIndexSemanticService>,
    ) -> Result<Self, CodeRetrievalError> {
        if index.root_id() != semantic.root_id() {
            return Err(CodeRetrievalError::RootMismatch);
        }
        Ok(Self {
            index,
            deployment: RetrievalDeployment::LocalSemantic(semantic),
            budget: CodeRetrievalBudget::default(),
        })
    }

    /// Creates a service with local lexical, local semantic, and optional remote semantic sources.
    pub fn local_semantic_with_cloud(
        index: Arc<CodeIndex>,
        semantic: Arc<CodeIndexSemanticService>,
        cloud: Arc<CloudCodeIndexController>,
    ) -> Result<Self, CodeRetrievalError> {
        if index.root_id() != semantic.root_id() || index.root_id() != cloud.root_id() {
            return Err(CodeRetrievalError::RootMismatch);
        }
        Ok(Self {
            index,
            deployment: RetrievalDeployment::Hybrid { semantic, cloud },
            budget: CodeRetrievalBudget::default(),
        })
    }

    pub fn with_budget(mut self, budget: CodeRetrievalBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Retrieves, fuses, verifies, deduplicates, and bounds code excerpts for Agent context.
    pub fn retrieve(
        &self,
        query: &CodeRetrievalQuery,
    ) -> Result<CodeRetrievalResult, CodeRetrievalError> {
        self.retrieve_with_cancellation(query, &CancellationSource::new().token())
    }

    /// Retrieves while forwarding cancellation into local semantic model calls.
    pub fn retrieve_with_cancellation(
        &self,
        query: &CodeRetrievalQuery,
        cancellation: &CancellationToken,
    ) -> Result<CodeRetrievalResult, CodeRetrievalError> {
        check_cancelled(cancellation)?;
        let candidate_limit = query
            .result_limit()
            .get()
            .saturating_mul(CANDIDATE_MULTIPLIER)
            .min(MAX_CANDIDATES_PER_SOURCE);
        let candidate_limit = NonZeroUsize::new(candidate_limit)
            .expect("a non-zero result limit produces a non-zero candidate limit");
        let local_query = CodeIndexQuery::new(query.text()).with_result_limit(candidate_limit);
        let local = self.index.search(&local_query)?;
        let mut fused = BTreeMap::<ChunkReference, FusedCandidate>::new();
        add_ranked(
            &mut fused,
            local.iter().map(|hit| hit.reference.clone()),
            CodeRetrievalOrigin::LocalLexical,
        );

        let mut degradations = Vec::new();
        let semantic = match &self.deployment {
            RetrievalDeployment::LocalSemantic(semantic)
            | RetrievalDeployment::Hybrid { semantic, .. } => Some(semantic),
            RetrievalDeployment::LocalOnly | RetrievalDeployment::Cloud(_) => None,
        };
        if let Some(semantic) = semantic {
            let semantic_query = CodeIndexSemanticQuery::new(query.text(), candidate_limit)
                .expect("retrieval query has already been validated");
            match semantic.query_with_cancellation(&semantic_query, cancellation) {
                Ok(result) => add_ranked(
                    &mut fused,
                    result.candidates,
                    CodeRetrievalOrigin::LocalSemantic,
                ),
                Err(_) => degradations.push(CodeRetrievalDegradation::LocalSemanticQueryFailed),
            }
        }

        let cloud = match &self.deployment {
            RetrievalDeployment::Cloud(cloud) | RetrievalDeployment::Hybrid { cloud, .. } => {
                Some(cloud)
            }
            RetrievalDeployment::LocalOnly | RetrievalDeployment::LocalSemantic(_) => None,
        };
        if let Some(cloud) = cloud {
            let cloud_query = CloudCodeIndexQuery::new(query.text(), candidate_limit)
                .expect("retrieval query has already been validated");
            match cloud.query(&cloud_query) {
                Ok(result) => {
                    add_ranked(
                        &mut fused,
                        result
                            .candidates
                            .iter()
                            .map(|candidate| candidate.reference.clone()),
                        CodeRetrievalOrigin::CloudSemantic,
                    );
                }
                Err(_) => degradations.push(CodeRetrievalDegradation::CloudQueryFailed),
            }
        }

        let mut ranked = fused.into_values().collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .rrf_score
                .total_cmp(&left.rrf_score)
                .then_with(|| left.reference.cmp(&right.reference))
        });
        let mut hits = Vec::with_capacity(query.result_limit().get());
        let mut verification_discarded = 0usize;
        let mut budget_discarded = 0usize;
        let mut total_bytes = 0usize;
        for candidate in ranked {
            check_cancelled(cancellation)?;
            if hits.len() == query.result_limit().get() {
                break;
            }
            let Ok(materialized) = self.index.materialize(&candidate.reference) else {
                verification_discarded = verification_discarded.saturating_add(1);
                continue;
            };
            let content_bytes = materialized.content.len();
            if content_bytes > self.budget.max_item_bytes()
                || total_bytes.saturating_add(content_bytes) > self.budget.max_total_bytes()
            {
                budget_discarded = budget_discarded.saturating_add(1);
                continue;
            }
            total_bytes = total_bytes.saturating_add(content_bytes);
            hits.push(CodeRetrievalHit {
                reference: SourceExcerptReference::from(&materialized.reference),
                language: materialized.language,
                content: materialized.content,
                rrf_score: candidate.rrf_score,
                origins: candidate.origins,
            });
        }
        if verification_discarded > 0 {
            degradations.push(CodeRetrievalDegradation::CandidateVerificationFailed {
                discarded: verification_discarded,
            });
        }
        if budget_discarded > 0 {
            degradations.push(CodeRetrievalDegradation::ContentBudgetExceeded {
                discarded: budget_discarded,
            });
        }
        Ok(CodeRetrievalResult { hits, degradations })
    }
}

fn check_cancelled(cancellation: &CancellationToken) -> Result<(), CodeRetrievalError> {
    cancellation
        .check()
        .map_err(|signal| CodeRetrievalError::Cancelled(signal.reason().to_string()))
}

struct FusedCandidate {
    reference: ChunkReference,
    rrf_score: f64,
    origins: Vec<CodeRetrievalOrigin>,
}

fn add_ranked(
    fused: &mut BTreeMap<ChunkReference, FusedCandidate>,
    candidates: impl IntoIterator<Item = ChunkReference>,
    origin: CodeRetrievalOrigin,
) {
    for (rank, reference) in candidates.into_iter().enumerate() {
        let rrf_score = 1.0 / (RRF_RANK_CONSTANT + rank as f64 + 1.0);
        let candidate = fused
            .entry(reference.clone())
            .or_insert_with(|| FusedCandidate {
                reference,
                rrf_score: 0.0,
                origins: Vec::new(),
            });
        candidate.rrf_score += rrf_score;
        if !candidate.origins.contains(&origin) {
            candidate.origins.push(origin);
            candidate.origins.sort_unstable();
        }
    }
}
