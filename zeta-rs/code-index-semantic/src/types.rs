use std::num::NonZeroUsize;
use std::path::PathBuf;

use zeta_code_index::ChunkReference;
use zeta_code_index::IndexedLanguage;
use zeta_model_provider::EmbeddingVector;

use crate::CodeIndexSemanticError;

macro_rules! text_identity {
    ($name:ident, $message:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CodeIndexSemanticError> {
                let value = value.into();
                if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                    return Err(CodeIndexSemanticError::InvalidInput($message));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_identity!(
    CodeIndexEmbeddingModelId,
    "embedding model identity must be 1..=256 bytes without control characters"
);

/// Storage placement for a rebuildable local semantic projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeIndexSemanticStorage {
    Memory,
    Persistent(PathBuf),
}

/// Bounded semantic query against the current local Workspace generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeIndexSemanticQuery {
    text: String,
    result_limit: NonZeroUsize,
}

impl CodeIndexSemanticQuery {
    pub fn new(
        text: impl Into<String>,
        result_limit: NonZeroUsize,
    ) -> Result<Self, CodeIndexSemanticError> {
        let text = text.into();
        if text.trim().is_empty() || text.len() > 8 * 1024 {
            return Err(CodeIndexSemanticError::InvalidInput(
                "query must contain 1..=8192 bytes of non-whitespace text",
            ));
        }
        if result_limit.get() > 100 {
            return Err(CodeIndexSemanticError::InvalidInput(
                "query result limit must not exceed 100",
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

/// Final locally ranked references for one exact semantic generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeIndexSemanticQueryResult {
    pub generation: u64,
    pub candidates: Vec<ChunkReference>,
}

/// Result of synchronizing the local semantic projection to the lexical index generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeIndexSemanticSyncResult {
    pub generation: u64,
    pub indexed_chunk_count: usize,
    pub reused_embedding_count: usize,
    pub retry_count: usize,
}

/// Stable phase of one semantic synchronization operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeIndexSemanticSyncPhase {
    Preparing,
    Embedding,
    Publishing,
    Complete,
}

/// Content-free progress counters for one exact lexical generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeIndexSemanticSyncProgress {
    pub phase: CodeIndexSemanticSyncPhase,
    pub generation: u64,
    pub total_chunk_count: usize,
    pub processed_chunk_count: usize,
    pub reused_embedding_count: usize,
    pub embedded_chunk_count: usize,
    pub completed_batch_count: usize,
    pub total_batch_count: usize,
    pub retry_count: usize,
}

/// Receives bounded, content-free semantic synchronization progress.
pub trait CodeIndexSemanticProgressSink: Send + Sync {
    fn report(&self, progress: &CodeIndexSemanticSyncProgress);
}

/// Privacy-safe operational measurement emitted by the semantic pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeIndexSemanticMetric {
    SyncCompleted {
        chunk_count: usize,
        reused_count: usize,
        embedded_count: usize,
        retry_count: usize,
        elapsed_millis: u64,
    },
    SyncCancelled {
        processed_count: usize,
    },
    SyncFailed,
    QueryCompleted {
        candidate_count: usize,
        retry_count: usize,
        elapsed_millis: u64,
    },
    QueryDegraded,
}

/// Receives metrics that intentionally exclude source text, paths, queries, endpoints, and secrets.
pub trait CodeIndexSemanticMetricsSink: Send + Sync {
    fn record(&self, metric: CodeIndexSemanticMetric);
}

/// One Workspace-produced chunk plus its model embedding stored by the local projection.
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedCodeChunk {
    pub reference: ChunkReference,
    pub language: IndexedLanguage,
    pub content: String,
    pub embedding: EmbeddingVector,
}

/// One vector-recall candidate in vector-store relevance order.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorSearchHit {
    pub chunk: EmbeddedCodeChunk,
    pub similarity: f32,
}
