use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use zeta_code_index::ChunkReference;
use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexQuery;
use zeta_code_index::SourceExcerptReference;
use zeta_code_index_cloud::CloudCodeIndexController;
use zeta_code_index_cloud::CloudCodeIndexQuery;

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
    Hybrid(Arc<CloudCodeIndexController>),
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
            deployment: RetrievalDeployment::Hybrid(cloud),
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
        if let RetrievalDeployment::Hybrid(cloud) = &self.deployment {
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
