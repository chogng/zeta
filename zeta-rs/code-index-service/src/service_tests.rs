use std::num::NonZeroUsize;
use std::sync::Arc;

use tempfile::TempDir;
use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexLimits;
use zeta_code_index::CodeIndexStorage;
use zeta_code_index::MaterializedChunk;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider::RerankInvoker;
use zeta_model_provider::RerankRequest;
use zeta_model_provider::RerankResponse;
use zeta_workspace::WorkspaceRoot;

use super::*;

struct KeywordEmbedding;

impl EmbeddingInvoker for KeywordEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        EmbeddingResponse::new(
            request
                .inputs()
                .iter()
                .map(|input| {
                    if input.contains("alpha") {
                        EmbeddingVector::new(vec![1.0, 0.0])
                    } else {
                        EmbeddingVector::new(vec![0.0, 1.0])
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    }
}

struct PreferAlphaRerank;

impl RerankInvoker for PreferAlphaRerank {
    fn rerank(&self, request: &RerankRequest) -> Result<RerankResponse, ModelProviderError> {
        RerankResponse::new(
            request
                .documents()
                .iter()
                .map(|document| if document.contains("alpha") { 1.0 } else { 0.0 })
                .collect(),
        )
    }
}

struct WrongCardinalityEmbedding;

impl EmbeddingInvoker for WrongCardinalityEmbedding {
    fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        EmbeddingResponse::new(vec![EmbeddingVector::new(vec![1.0, 0.0])?])
    }
}

#[test]
fn vector_recall_returns_workspace_chunk_references_in_similarity_order() {
    let chunks = workspace_chunks();
    let service = service(None);
    service
        .publish(publication(chunks.clone()))
        .expect("publish");

    let result = service.query(&query("find beta")).expect("semantic query");

    assert_eq!(result.candidates.len(), 2);
    assert_eq!(
        result.candidates[0].relative_path,
        std::path::Path::new("beta.rs")
    );
    assert!(
        chunks
            .iter()
            .any(|chunk| chunk.reference == result.candidates[0])
    );
}

#[test]
fn code_index_service_interprets_rerank_scores_and_owns_final_order() {
    let chunks = workspace_chunks();
    let rerank: Arc<dyn RerankInvoker> = Arc::new(PreferAlphaRerank);
    let service = service(Some(rerank));
    service.publish(publication(chunks)).expect("publish");

    let result = service.query(&query("find beta")).expect("semantic query");

    assert_eq!(
        result.candidates[0].relative_path,
        std::path::Path::new("alpha.rs")
    );
}

#[test]
fn publication_rejects_model_output_that_does_not_match_workspace_chunks() {
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(WrongCardinalityEmbedding);
    let store: Arc<dyn CodeIndexVectorStore> = Arc::new(InMemoryCodeIndexVectorStore::default());
    let service = CodeIndexSemanticService::new(embedding, None, store);

    assert!(matches!(
        service.publish(publication(workspace_chunks())),
        Err(CodeIndexServiceError::InvalidModelResponse(
            "embedding count does not match the published chunk count"
        ))
    ));
}

#[test]
fn publication_rejects_duplicate_workspace_chunk_identity() {
    let mut chunks = workspace_chunks();
    chunks.push(chunks[0].clone());

    assert!(matches!(
        service(None).publish(publication(chunks)),
        Err(CodeIndexServiceError::InvalidInput(
            "publication contains duplicate Workspace chunk references"
        ))
    ));
}

#[test]
fn publication_rejects_chunks_from_different_workspace_roots() {
    let mut chunks = workspace_chunks();
    let another = workspace_chunks();
    chunks.push(another[0].clone());

    assert!(matches!(
        service(None).publish(publication(chunks)),
        Err(CodeIndexServiceError::InvalidInput(
            "publication must contain chunks from one Workspace root"
        ))
    ));
}

fn service(rerank: Option<Arc<dyn RerankInvoker>>) -> CodeIndexSemanticService {
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(KeywordEmbedding);
    let store: Arc<dyn CodeIndexVectorStore> = Arc::new(InMemoryCodeIndexVectorStore::default());
    CodeIndexSemanticService::new(embedding, rerank, store)
}

fn publication(chunks: Vec<MaterializedChunk>) -> CodeIndexSemanticPublication {
    CodeIndexSemanticPublication {
        collection: collection(),
        generation: generation(),
        chunks,
    }
}

fn query(text: &str) -> CodeIndexSemanticQuery {
    CodeIndexSemanticQuery::new(
        collection(),
        generation(),
        text,
        NonZeroUsize::new(10).expect("limit"),
    )
    .expect("query")
}

fn collection() -> CodeIndexCollectionId {
    CodeIndexCollectionId::new("collection-a").expect("collection")
}

fn generation() -> CodeIndexGenerationId {
    CodeIndexGenerationId::new("generation-1").expect("generation")
}

fn workspace_chunks() -> Vec<MaterializedChunk> {
    let workspace = workspace();
    std::fs::write(
        workspace.path().join("alpha.rs"),
        "pub fn alpha_feature() {}\n",
    )
    .expect("alpha source");
    std::fs::write(
        workspace.path().join("beta.rs"),
        "pub fn beta_feature() {}\n",
    )
    .expect("beta source");
    let index = CodeIndex::open(
        WorkspaceRoot::open(workspace.path()).expect("root"),
        CodeIndexStorage::Memory,
        CodeIndexLimits::default(),
    )
    .expect("index");
    index.rebuild().expect("rebuild");
    let manifest = index.manifest().expect("manifest");
    index
        .materialize_chunks(&manifest.chunks)
        .expect("materialize chunks")
}

fn workspace() -> TempDir {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("git marker");
    workspace
}
