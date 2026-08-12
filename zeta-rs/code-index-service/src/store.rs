use std::num::NonZeroUsize;

use zeta_model_provider::EmbeddingVector;

use crate::CodeIndexCollectionId;
use crate::CodeIndexGenerationId;
use crate::CodeIndexVectorStoreError;
use crate::EmbeddedCodeChunk;
use crate::VectorSearchHit;

/// Exact-generation vector persistence and nearest-neighbor primitive used by CodeIndex.
///
/// Implementations must atomically replace one collection generation, never merge references
/// across generations, preserve Workspace chunk identities, and make deletion idempotent.
/// Similarity search returns candidates in descending relevance order; final rerank policy remains
/// with [`crate::CodeIndexSemanticService`].
pub trait CodeIndexVectorStore: Send + Sync {
    fn replace_generation(
        &self,
        collection: &CodeIndexCollectionId,
        generation: &CodeIndexGenerationId,
        chunks: Vec<EmbeddedCodeChunk>,
    ) -> Result<(), CodeIndexVectorStoreError>;

    fn search(
        &self,
        collection: &CodeIndexCollectionId,
        generation: &CodeIndexGenerationId,
        query: &EmbeddingVector,
        result_limit: NonZeroUsize,
    ) -> Result<Vec<VectorSearchHit>, CodeIndexVectorStoreError>;

    fn delete_collection(
        &self,
        collection: &CodeIndexCollectionId,
    ) -> Result<(), CodeIndexVectorStoreError>;
}
