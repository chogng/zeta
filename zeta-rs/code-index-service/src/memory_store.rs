use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use zeta_model_provider::EmbeddingVector;

use crate::CodeIndexCollectionId;
use crate::CodeIndexGenerationId;
use crate::CodeIndexVectorStore;
use crate::CodeIndexVectorStoreError;
use crate::EmbeddedCodeChunk;
use crate::VectorSearchHit;

#[derive(Default)]
pub struct InMemoryCodeIndexVectorStore {
    collections: Mutex<BTreeMap<CodeIndexCollectionId, StoredGeneration>>,
}

struct StoredGeneration {
    id: CodeIndexGenerationId,
    chunks: Vec<EmbeddedCodeChunk>,
}

impl CodeIndexVectorStore for InMemoryCodeIndexVectorStore {
    fn replace_generation(
        &self,
        collection: &CodeIndexCollectionId,
        generation: &CodeIndexGenerationId,
        chunks: Vec<EmbeddedCodeChunk>,
    ) -> Result<(), CodeIndexVectorStoreError> {
        let dimension = chunks.first().map(|chunk| chunk.embedding.values().len());
        if dimension.is_some_and(|dimension| {
            chunks
                .iter()
                .any(|chunk| chunk.embedding.values().len() != dimension)
        }) {
            return Err(CodeIndexVectorStoreError::new(
                "stored embedding dimensions are inconsistent",
            ));
        }
        self.collections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                collection.clone(),
                StoredGeneration {
                    id: generation.clone(),
                    chunks,
                },
            );
        Ok(())
    }

    fn search(
        &self,
        collection: &CodeIndexCollectionId,
        generation: &CodeIndexGenerationId,
        query: &EmbeddingVector,
        result_limit: NonZeroUsize,
    ) -> Result<Vec<VectorSearchHit>, CodeIndexVectorStoreError> {
        let collections = self
            .collections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let stored = collections
            .get(collection)
            .ok_or_else(|| CodeIndexVectorStoreError::new("collection does not exist"))?;
        if &stored.id != generation {
            return Err(CodeIndexVectorStoreError::new(
                "requested generation is not current",
            ));
        }
        let mut hits = stored
            .chunks
            .iter()
            .cloned()
            .map(|chunk| {
                let similarity = cosine_similarity(query.values(), chunk.embedding.values())?;
                Ok(VectorSearchHit { chunk, similarity })
            })
            .collect::<Result<Vec<_>, CodeIndexVectorStoreError>>()?;
        hits.sort_by(|left, right| {
            right
                .similarity
                .total_cmp(&left.similarity)
                .then_with(|| left.chunk.reference.cmp(&right.chunk.reference))
        });
        hits.truncate(result_limit.get());
        Ok(hits)
    }

    fn delete_collection(
        &self,
        collection: &CodeIndexCollectionId,
    ) -> Result<(), CodeIndexVectorStoreError> {
        self.collections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(collection);
        Ok(())
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32, CodeIndexVectorStoreError> {
    if left.len() != right.len() {
        return Err(CodeIndexVectorStoreError::new(
            "query and stored embedding dimensions differ",
        ));
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        Ok(0.0)
    } else {
        Ok(dot / (left_norm * right_norm))
    }
}
