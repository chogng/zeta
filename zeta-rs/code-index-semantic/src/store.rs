use std::num::NonZeroUsize;

use zeta_code_index::MaterializedChunk;
use zeta_model_provider::EmbeddingVector;

use crate::CodeIndexEmbeddingModelId;
use crate::CodeIndexVectorStoreError;
use crate::EmbeddedCodeChunk;
use crate::VectorSearchHit;

/// Exact-generation vector persistence and nearest-neighbor primitive used by CodeIndex.
///
/// Implementations must atomically replace one Workspace generation, never merge references
/// across generations or embedding models, preserve Workspace chunk identities, and make deletion idempotent.
/// Similarity search returns candidates in descending relevance order; final rerank policy remains
/// with [`crate::CodeIndexSemanticService`].
pub trait CodeIndexVectorStore: Send + Sync {
    /// Loads reusable embeddings for stable chunks in input order.
    ///
    /// Implementations may reuse a vector only when the Workspace root, embedding model, relative
    /// path, language, and stable chunk key match. A generation mismatch alone must not prevent
    /// reuse because lexical generations advance after restart and rebuild.
    fn reusable_embeddings(
        &self,
        root_id: &zeta_code_index::IndexRootId,
        embedding_model: &CodeIndexEmbeddingModelId,
        chunks: &[MaterializedChunk],
    ) -> Result<Vec<Option<EmbeddingVector>>, CodeIndexVectorStoreError>;

    /// Saves completed embedding batches before a generation is published.
    ///
    /// Implementations must key cached vectors by the exact root, model, path, language, stable
    /// chunk identity, and content identity. Cached vectors are rebuildable and must never make a
    /// partial generation searchable.
    fn cache_embeddings(
        &self,
        root_id: &zeta_code_index::IndexRootId,
        embedding_model: &CodeIndexEmbeddingModelId,
        chunks: &[EmbeddedCodeChunk],
    ) -> Result<(), CodeIndexVectorStoreError>;

    /// Returns the currently published searchable generation, if any.
    fn published_generation(
        &self,
        root_id: &zeta_code_index::IndexRootId,
        embedding_model: &CodeIndexEmbeddingModelId,
    ) -> Result<Option<u64>, CodeIndexVectorStoreError>;

    /// Atomically publishes all chunks for one exact lexical generation.
    fn replace_generation(
        &self,
        root_id: &zeta_code_index::IndexRootId,
        generation: u64,
        embedding_model: &CodeIndexEmbeddingModelId,
        chunks: Vec<EmbeddedCodeChunk>,
    ) -> Result<(), CodeIndexVectorStoreError>;

    /// Returns descending-similarity candidates only for the exact root, generation, and model.
    fn search(
        &self,
        root_id: &zeta_code_index::IndexRootId,
        generation: u64,
        embedding_model: &CodeIndexEmbeddingModelId,
        query: &EmbeddingVector,
        result_limit: NonZeroUsize,
    ) -> Result<Vec<VectorSearchHit>, CodeIndexVectorStoreError>;

    /// Deletes the rebuildable projection for the given root and succeeds when it is already gone.
    fn delete_index(
        &self,
        root_id: &zeta_code_index::IndexRootId,
    ) -> Result<(), CodeIndexVectorStoreError>;
}
