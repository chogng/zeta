//! Local semantic indexing for Workspace-produced code chunks.
//!
//! The crate embeds exact chunks from `zeta-code-index`, persists vectors in a rebuildable local
//! projection, owns vector recall and optional rerank ordering, and returns chunk references only.
//! It never scans a Workspace or owns remote service deployment.

mod error;
mod memory_store;
mod service;
mod sqlite_store;
mod store;
mod types;

pub use error::CodeIndexSemanticError;
pub use error::CodeIndexVectorStoreError;
pub use memory_store::InMemoryCodeIndexVectorStore;
pub use service::CodeIndexSemanticService;
pub use sqlite_store::SqliteCodeIndexVectorStore;
pub use store::CodeIndexVectorStore;
pub use types::CodeIndexEmbeddingModelId;
pub use types::CodeIndexSemanticMetric;
pub use types::CodeIndexSemanticMetricsSink;
pub use types::CodeIndexSemanticProgressSink;
pub use types::CodeIndexSemanticQuery;
pub use types::CodeIndexSemanticQueryResult;
pub use types::CodeIndexSemanticStorage;
pub use types::CodeIndexSemanticSyncPhase;
pub use types::CodeIndexSemanticSyncProgress;
pub use types::CodeIndexSemanticSyncResult;
pub use types::EmbeddedCodeChunk;
pub use types::VectorSearchHit;

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
