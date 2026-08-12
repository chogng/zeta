use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::sync::Arc;

use zeta_code_index::MaterializedChunk;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::RerankInvoker;
use zeta_model_provider::RerankRequest;

use crate::CodeIndexCollectionId;
use crate::CodeIndexSemanticPublication;
use crate::CodeIndexSemanticQuery;
use crate::CodeIndexSemanticQueryResult;
use crate::CodeIndexServiceError;
use crate::CodeIndexVectorStore;
use crate::EmbeddedCodeChunk;

const EMBEDDING_BATCH_SIZE: usize = 128;
const VECTOR_RECALL_MULTIPLIER: usize = 4;
const MAX_VECTOR_RECALL: usize = 400;

/// Owns semantic indexing, vector recall, optional rerank, and final candidate ordering.
pub struct CodeIndexSemanticService {
    embedding: Arc<dyn EmbeddingInvoker>,
    rerank: Option<Arc<dyn RerankInvoker>>,
    store: Arc<dyn CodeIndexVectorStore>,
}

impl CodeIndexSemanticService {
    pub fn new(
        embedding: Arc<dyn EmbeddingInvoker>,
        rerank: Option<Arc<dyn RerankInvoker>>,
        store: Arc<dyn CodeIndexVectorStore>,
    ) -> Self {
        Self {
            embedding,
            rerank,
            store,
        }
    }

    pub fn publish(
        &self,
        publication: CodeIndexSemanticPublication,
    ) -> Result<(), CodeIndexServiceError> {
        validate_publication(&publication.chunks)?;
        let mut embedded = Vec::with_capacity(publication.chunks.len());
        for batch in publication.chunks.chunks(EMBEDDING_BATCH_SIZE) {
            let inputs = batch.iter().map(code_document).collect::<Vec<_>>();
            let response = self.embedding.embed(&EmbeddingRequest::new(inputs)?)?;
            if response.vectors().len() != batch.len() {
                return Err(CodeIndexServiceError::InvalidModelResponse(
                    "embedding count does not match the published chunk count",
                ));
            }
            embedded.extend(batch.iter().cloned().zip(response.into_vectors()).map(
                |(chunk, embedding)| EmbeddedCodeChunk {
                    reference: chunk.reference,
                    language: chunk.language,
                    content: chunk.content,
                    embedding,
                },
            ));
        }
        self.store.replace_generation(
            &publication.collection,
            &publication.generation,
            embedded,
        )?;
        Ok(())
    }

    pub fn query(
        &self,
        query: &CodeIndexSemanticQuery,
    ) -> Result<CodeIndexSemanticQueryResult, CodeIndexServiceError> {
        let response = self
            .embedding
            .embed(&EmbeddingRequest::new(vec![query.text().to_owned()])?)?;
        let mut vectors = response.into_vectors();
        if vectors.len() != 1 {
            return Err(CodeIndexServiceError::InvalidModelResponse(
                "query embedding response must contain exactly one vector",
            ));
        }
        let recall_limit = query
            .result_limit()
            .get()
            .saturating_mul(VECTOR_RECALL_MULTIPLIER)
            .min(MAX_VECTOR_RECALL);
        let recall_limit = NonZeroUsize::new(recall_limit).expect("query result limit is non-zero");
        let mut candidates = self.store.search(
            &query.collection,
            &query.generation,
            &vectors.remove(0),
            recall_limit,
        )?;
        if let Some(rerank) = &self.rerank
            && !candidates.is_empty()
        {
            let documents = candidates
                .iter()
                .map(|candidate| {
                    code_document_from_parts(
                        &candidate.chunk.reference.relative_path,
                        candidate.chunk.language,
                        &candidate.chunk.content,
                    )
                })
                .collect();
            let response = rerank.rerank(&RerankRequest::new(query.text(), documents)?)?;
            if response.scores().len() != candidates.len() {
                return Err(CodeIndexServiceError::InvalidModelResponse(
                    "rerank score count does not match the recalled candidate count",
                ));
            }
            let mut reranked = candidates
                .into_iter()
                .zip(response.scores().iter().copied())
                .enumerate()
                .collect::<Vec<_>>();
            reranked.sort_by(|left, right| {
                right
                    .1
                    .1
                    .total_cmp(&left.1.1)
                    .then_with(|| left.0.cmp(&right.0))
                    .then_with(|| left.1.0.chunk.reference.cmp(&right.1.0.chunk.reference))
            });
            candidates = reranked
                .into_iter()
                .map(|(_, (candidate, _))| candidate)
                .collect();
        }
        candidates.truncate(query.result_limit().get());
        Ok(CodeIndexSemanticQueryResult {
            generation: query.generation.clone(),
            candidates: candidates
                .into_iter()
                .map(|candidate| candidate.chunk.reference)
                .collect(),
        })
    }

    pub fn delete_collection(
        &self,
        collection: &CodeIndexCollectionId,
    ) -> Result<(), CodeIndexServiceError> {
        self.store.delete_collection(collection)?;
        Ok(())
    }
}

fn validate_publication(chunks: &[MaterializedChunk]) -> Result<(), CodeIndexServiceError> {
    let roots = chunks
        .iter()
        .map(|chunk| &chunk.reference.root_id)
        .collect::<BTreeSet<_>>();
    if roots.len() > 1 {
        return Err(CodeIndexServiceError::InvalidInput(
            "publication must contain chunks from one Workspace root",
        ));
    }
    let mut references = BTreeSet::new();
    if chunks
        .iter()
        .any(|chunk| !references.insert(chunk.reference.clone()))
    {
        return Err(CodeIndexServiceError::InvalidInput(
            "publication contains duplicate Workspace chunk references",
        ));
    }
    Ok(())
}

fn code_document(chunk: &MaterializedChunk) -> String {
    code_document_from_parts(
        &chunk.reference.relative_path,
        chunk.language,
        &chunk.content,
    )
}

fn code_document_from_parts(
    path: &std::path::Path,
    language: zeta_code_index::IndexedLanguage,
    content: &str,
) -> String {
    format!(
        "path: {}\nlanguage: {language:?}\n\n{content}",
        path.display()
    )
}
