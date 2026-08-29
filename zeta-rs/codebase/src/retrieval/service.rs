use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use crate::ChunkSpan;
use crate::Codebase;
use crate::CodebaseQuery;
use crate::CodebaseSemanticQuery;
use crate::CodebaseSemanticService;
use crate::IndexedSourceReference;
use crate::SourceExcerptReference;
use crate::SymbolIndex;
use crate::SymbolIndexQuery;
use crate::SymbolReference;
use zeta_async_utils::CancellationSource;
use zeta_async_utils::CancellationToken;

use crate::CodebaseEnhancement;
use crate::CodebaseRetrievalBudget;
use crate::CodebaseRetrievalDegradation;
use crate::CodebaseRetrievalError;
use crate::CodebaseRetrievalHit;
use crate::CodebaseRetrievalOrigin;
use crate::CodebaseRetrievalQuery;
use crate::CodebaseRetrievalResult;

const RRF_RANK_CONSTANT: f64 = 60.0;
const CANDIDATE_MULTIPLIER: usize = 4;
const MAX_CANDIDATES_PER_SOURCE: usize = 100;

enum RetrievalDeployment {
    LocalOnly,
    LocalSemantic(Arc<CodebaseSemanticService>),
    Enhanced(Arc<dyn CodebaseEnhancement>),
}

/// Workspace-scoped local/cloud candidate coordinator.
pub struct CodebaseRetrievalService {
    index: Arc<Codebase>,
    symbol_index: Option<Arc<SymbolIndex>>,
    deployment: RetrievalDeployment,
    budget: CodebaseRetrievalBudget,
}

impl CodebaseRetrievalService {
    /// Creates a service that never invokes a cloud provider.
    pub fn local(index: Arc<Codebase>) -> Self {
        Self {
            index,
            symbol_index: None,
            deployment: RetrievalDeployment::LocalOnly,
            budget: CodebaseRetrievalBudget::default(),
        }
    }

    /// Creates a service that fuses cloud candidates and falls back to local candidates on any
    /// non-fatal cloud query failure.
    pub fn enhanced(
        index: Arc<Codebase>,
        enhancement: Arc<dyn CodebaseEnhancement>,
    ) -> Result<Self, CodebaseRetrievalError> {
        if index.root_id() != enhancement.root_id() {
            return Err(CodebaseRetrievalError::RootMismatch);
        }
        Ok(Self {
            index,
            symbol_index: None,
            deployment: RetrievalDeployment::Enhanced(enhancement),
            budget: CodebaseRetrievalBudget::default(),
        })
    }

    /// Creates a local-first service that fuses lexical and local semantic candidates.
    pub fn local_semantic(
        index: Arc<Codebase>,
        semantic: Arc<CodebaseSemanticService>,
    ) -> Result<Self, CodebaseRetrievalError> {
        if index.root_id() != semantic.root_id() {
            return Err(CodebaseRetrievalError::RootMismatch);
        }
        Ok(Self {
            index,
            symbol_index: None,
            deployment: RetrievalDeployment::LocalSemantic(semantic),
            budget: CodebaseRetrievalBudget::default(),
        })
    }

    pub fn with_budget(mut self, budget: CodebaseRetrievalBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Adds the Workspace-owned declaration projection as another local candidate source.
    pub fn with_symbol_index(
        mut self,
        symbol_index: Arc<SymbolIndex>,
    ) -> Result<Self, CodebaseRetrievalError> {
        if self.index.root_id() != symbol_index.root_id() {
            return Err(CodebaseRetrievalError::RootMismatch);
        }
        self.symbol_index = Some(symbol_index);
        Ok(self)
    }

    /// Retrieves, fuses, verifies, deduplicates, and bounds code excerpts for Agent context.
    pub fn retrieve(
        &self,
        query: &CodebaseRetrievalQuery,
    ) -> Result<CodebaseRetrievalResult, CodebaseRetrievalError> {
        self.retrieve_with_cancellation(query, &CancellationSource::new().token())
    }

    /// Retrieves while forwarding cancellation into local semantic model calls.
    pub fn retrieve_with_cancellation(
        &self,
        query: &CodebaseRetrievalQuery,
        cancellation: &CancellationToken,
    ) -> Result<CodebaseRetrievalResult, CodebaseRetrievalError> {
        check_cancelled(cancellation)?;
        let candidate_limit = query
            .result_limit()
            .get()
            .saturating_mul(CANDIDATE_MULTIPLIER)
            .min(MAX_CANDIDATES_PER_SOURCE);
        let candidate_limit = NonZeroUsize::new(candidate_limit)
            .expect("a non-zero result limit produces a non-zero candidate limit");
        let local_query = CodebaseQuery::new(query.text()).with_result_limit(candidate_limit);
        let dirty_paths = self.index.dirty_overlay_paths();
        let local = self.index.search(&local_query)?;
        let mut fused = BTreeMap::<SourceExcerptReference, FusedCandidate>::new();
        add_ranked(
            &mut fused,
            local
                .iter()
                .map(|hit| SourceExcerptReference::from(&hit.reference)),
            CodebaseRetrievalOrigin::LocalLexical,
        );

        let mut degradations = Vec::new();
        let mut symbol_verification_discarded = 0usize;
        if let Some(symbol_index) = &self.symbol_index {
            let symbol_query =
                SymbolIndexQuery::new(query.text()).with_result_limit(candidate_limit);
            match symbol_index.search(&symbol_query) {
                Ok(hits) => {
                    let mut references = Vec::with_capacity(hits.len());
                    for hit in hits {
                        let source = indexed_source(&hit.symbol.reference);
                        let span = declaration_span(&hit.symbol.reference);
                        match self.index.materialize_verified_excerpt(&source, span) {
                            Ok(materialized) => references.push(materialized.reference),
                            Err(_) => {
                                symbol_verification_discarded =
                                    symbol_verification_discarded.saturating_add(1);
                            }
                        }
                    }
                    add_ranked(&mut fused, references, CodebaseRetrievalOrigin::LocalSymbol);
                }
                Err(_) => degradations.push(CodebaseRetrievalDegradation::LocalSymbolQueryFailed),
            }
        }
        let semantic = match &self.deployment {
            RetrievalDeployment::LocalSemantic(semantic) => Some(semantic),
            RetrievalDeployment::LocalOnly | RetrievalDeployment::Enhanced(_) => None,
        };
        if let Some(semantic) = semantic {
            let semantic_query = CodebaseSemanticQuery::new(query.text(), candidate_limit)
                .expect("retrieval query has already been validated");
            match semantic.query_with_cancellation(&semantic_query, cancellation) {
                Ok(result) => add_ranked(
                    &mut fused,
                    result
                        .candidates
                        .into_iter()
                        .filter(|reference| !dirty_paths.contains(&reference.relative_path))
                        .map(|reference| SourceExcerptReference::from(&reference)),
                    CodebaseRetrievalOrigin::LocalSemantic,
                ),
                Err(_) => degradations.push(CodebaseRetrievalDegradation::LocalSemanticQueryFailed),
            }
        }

        let enhancement = match &self.deployment {
            RetrievalDeployment::Enhanced(enhancement) => Some(enhancement),
            RetrievalDeployment::LocalOnly | RetrievalDeployment::LocalSemantic(_) => None,
        };
        if let Some(enhancement) = enhancement {
            match enhancement.query(query.text(), candidate_limit) {
                Ok(result) => {
                    add_ranked(
                        &mut fused,
                        result
                            .iter()
                            .filter(|reference| !dirty_paths.contains(&reference.relative_path))
                            .map(SourceExcerptReference::from),
                        CodebaseRetrievalOrigin::CloudSemantic,
                    );
                }
                Err(_) => degradations.push(CodebaseRetrievalDegradation::CloudQueryFailed),
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
        let mut verification_discarded = symbol_verification_discarded;
        let mut budget_discarded = 0usize;
        let mut total_bytes = 0usize;
        for candidate in ranked {
            check_cancelled(cancellation)?;
            if hits.len() == query.result_limit().get() {
                break;
            }
            let Ok(materialized) = self.index.materialize_excerpt(&candidate.reference) else {
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
            hits.push(CodebaseRetrievalHit {
                reference: materialized.reference,
                language: materialized.language,
                content: materialized.content,
                rrf_score: candidate.rrf_score,
                origins: candidate.origins,
            });
        }
        if verification_discarded > 0 {
            degradations.push(CodebaseRetrievalDegradation::CandidateVerificationFailed {
                discarded: verification_discarded,
            });
        }
        if budget_discarded > 0 {
            degradations.push(CodebaseRetrievalDegradation::ContentBudgetExceeded {
                discarded: budget_discarded,
            });
        }
        Ok(CodebaseRetrievalResult { hits, degradations })
    }
}

fn check_cancelled(cancellation: &CancellationToken) -> Result<(), CodebaseRetrievalError> {
    cancellation
        .check()
        .map_err(|signal| CodebaseRetrievalError::Cancelled(signal.reason().to_string()))
}

struct FusedCandidate {
    reference: SourceExcerptReference,
    rrf_score: f64,
    origins: Vec<CodebaseRetrievalOrigin>,
}

fn add_ranked(
    fused: &mut BTreeMap<SourceExcerptReference, FusedCandidate>,
    candidates: impl IntoIterator<Item = SourceExcerptReference>,
    origin: CodebaseRetrievalOrigin,
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

fn indexed_source(reference: &SymbolReference) -> IndexedSourceReference {
    IndexedSourceReference {
        root_id: reference.root_id.clone(),
        relative_path: reference.relative_path.clone(),
        source_revision: reference.source_revision.clone(),
        language: reference.language,
        source_bytes: reference.source_bytes,
    }
}

fn declaration_span(reference: &SymbolReference) -> ChunkSpan {
    let range = &reference.declaration_range;
    let end_line_exclusive = if range.end_column == 0 && range.end_line > range.start_line {
        range.end_line
    } else {
        range.end_line.saturating_add(1)
    };
    ChunkSpan {
        start_byte: range.start_byte,
        end_byte: range.end_byte,
        start_line: range.start_line,
        end_line_exclusive,
    }
}
