//! Provider-neutral semantic indexing pipeline for Workspace-produced code chunks.
//!
//! The service receives exact chunks from a Workspace authority, invokes embedding and rerank
//! model APIs, owns vector recall and final candidate ordering, and returns chunk references only.
//! It has no filesystem, Workspace traversal, ignore, or chunking authority.

mod error;
mod memory_store;
mod service;
mod store;
mod types;

pub use error::CodeIndexServiceError;
pub use error::CodeIndexVectorStoreError;
pub use memory_store::InMemoryCodeIndexVectorStore;
pub use service::CodeIndexSemanticService;
pub use store::CodeIndexVectorStore;
pub use types::CodeIndexCollectionId;
pub use types::CodeIndexGenerationId;
pub use types::CodeIndexSemanticPublication;
pub use types::CodeIndexSemanticQuery;
pub use types::CodeIndexSemanticQueryResult;
pub use types::EmbeddedCodeChunk;
pub use types::VectorSearchHit;

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
