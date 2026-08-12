use std::num::NonZeroUsize;

use zeta_code_index::IndexedLanguage;
use zeta_code_index::SourceExcerptReference;

use crate::CodeRetrievalError;

const DEFAULT_MAX_ITEM_BYTES: usize = 32 * 1024;
const DEFAULT_MAX_TOTAL_BYTES: usize = 128 * 1024;

/// Literal query shared by local lexical and optional semantic candidate sources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeRetrievalQuery {
    text: String,
    result_limit: NonZeroUsize,
}

impl CodeRetrievalQuery {
    pub fn new(
        text: impl Into<String>,
        result_limit: NonZeroUsize,
    ) -> Result<Self, CodeRetrievalError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(CodeRetrievalError::InvalidQuery(
                "query must contain non-whitespace text",
            ));
        }
        if text.len() > 8 * 1024 {
            return Err(CodeRetrievalError::InvalidQuery(
                "query exceeds the 8192-byte limit",
            ));
        }
        if result_limit.get() > 100 {
            return Err(CodeRetrievalError::InvalidQuery(
                "result limit must not exceed 100",
            ));
        }
        Ok(Self { text, result_limit })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn result_limit(&self) -> NonZeroUsize {
        self.result_limit
    }
}

/// Content ceilings applied after candidate fusion and current-source verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeRetrievalBudget {
    max_item_bytes: NonZeroUsize,
    max_total_bytes: NonZeroUsize,
}

impl CodeRetrievalBudget {
    pub fn with_max_item_bytes(mut self, value: NonZeroUsize) -> Self {
        self.max_item_bytes = value;
        self
    }

    pub fn with_max_total_bytes(mut self, value: NonZeroUsize) -> Self {
        self.max_total_bytes = value;
        self
    }

    pub(crate) fn max_item_bytes(&self) -> usize {
        self.max_item_bytes.get()
    }

    pub(crate) fn max_total_bytes(&self) -> usize {
        self.max_total_bytes.get()
    }
}

impl Default for CodeRetrievalBudget {
    fn default() -> Self {
        Self {
            max_item_bytes: NonZeroUsize::new(DEFAULT_MAX_ITEM_BYTES)
                .expect("default item budget is non-zero"),
            max_total_bytes: NonZeroUsize::new(DEFAULT_MAX_TOTAL_BYTES)
                .expect("default total budget is non-zero"),
        }
    }
}

/// Candidate source that contributed to one fused hit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CodeRetrievalOrigin {
    LocalLexical,
    LocalSemantic,
    CloudSemantic,
}

/// Non-fatal loss of one optional source or candidate set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeRetrievalDegradation {
    LocalSemanticQueryFailed,
    CloudQueryFailed,
    CandidateVerificationFailed { discarded: usize },
    ContentBudgetExceeded { discarded: usize },
}

/// One deduplicated, current-source-verified code excerpt.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeRetrievalHit {
    pub reference: SourceExcerptReference,
    pub language: IndexedLanguage,
    pub content: String,
    pub rrf_score: f64,
    pub origins: Vec<CodeRetrievalOrigin>,
}

/// Fused hits plus explicit non-fatal degradation facts.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeRetrievalResult {
    pub hits: Vec<CodeRetrievalHit>,
    pub degradations: Vec<CodeRetrievalDegradation>,
}
