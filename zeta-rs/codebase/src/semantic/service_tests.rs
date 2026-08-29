use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::Codebase;
use crate::CodebaseLimits;
use crate::CodebaseStorage;
use tempfile::TempDir;
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

struct CountingEmbedding {
    embedded_input_count: Arc<AtomicUsize>,
}

struct CancellingBatchEmbedding {
    calls: AtomicUsize,
    cancellation: Mutex<Option<zeta_async_utils::CancellationSource>>,
}

struct TransientOnceEmbedding {
    calls: AtomicUsize,
}

#[derive(Default)]
struct RecordingMetrics {
    metrics: Mutex<Vec<CodebaseSemanticMetric>>,
}

impl CodebaseSemanticMetricsSink for RecordingMetrics {
    fn record(&self, metric: CodebaseSemanticMetric) {
        self.metrics.lock().unwrap().push(metric);
    }
}

impl EmbeddingInvoker for TransientOnceEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
            return Err(ModelProviderError::Api(zeta_api::ApiError::Transport(
                "temporary".into(),
            )));
        }
        KeywordEmbedding.embed(request)
    }
}

impl EmbeddingInvoker for CancellingBatchEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        if call == 0
            && let Some(cancellation) = self.cancellation.lock().unwrap().as_ref()
        {
            cancellation.cancel();
        }
        EmbeddingResponse::new(
            request
                .inputs()
                .iter()
                .map(|_| EmbeddingVector::new(vec![1.0, 0.0]))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }
}

impl EmbeddingInvoker for CountingEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        self.embedded_input_count
            .fetch_add(request.inputs().len(), Ordering::Relaxed);
        KeywordEmbedding.embed(request)
    }
}

#[test]
fn local_vector_recall_returns_current_workspace_chunk_references() {
    let workspace = workspace();
    let index = index(&workspace);
    let service = service(Arc::clone(&index), None, memory_store());
    let synced = service.sync().expect("sync");

    let result = service.query(&query("find beta")).expect("semantic query");

    assert_eq!(
        synced.generation,
        index.snapshot().expect("snapshot").generation
    );
    assert_eq!(result.generation, synced.generation);
    assert_eq!(result.candidates.len(), 2);
    assert_eq!(
        result.candidates[0].relative_path,
        std::path::Path::new("beta.rs")
    );
}

#[test]
fn local_semantic_service_interprets_rerank_scores_and_owns_final_order() {
    let workspace = workspace();
    let index = index(&workspace);
    let rerank: Arc<dyn RerankInvoker> = Arc::new(PreferAlphaRerank);
    let service = service(index, Some(rerank), memory_store());
    service.sync().expect("sync");

    let result = service.query(&query("find beta")).expect("semantic query");

    assert_eq!(
        result.candidates[0].relative_path,
        std::path::Path::new("alpha.rs")
    );
}

#[test]
fn persistent_sqlite_projection_reopens_without_reembedding_chunks() {
    let workspace = workspace();
    let index = index(&workspace);
    let storage = tempfile::tempdir().expect("storage");
    let path = storage.path().join("semantic.sqlite3");
    let first_store: Arc<dyn CodebaseVectorStore> = Arc::new(
        SqliteCodebaseVectorStore::open(&CodebaseSemanticStorage::Persistent(path.clone()))
            .expect("open store"),
    );
    service(Arc::clone(&index), None, first_store)
        .sync()
        .expect("sync");

    let reopened: Arc<dyn CodebaseVectorStore> = Arc::new(
        SqliteCodebaseVectorStore::open(&CodebaseSemanticStorage::Persistent(path))
            .expect("reopen store"),
    );
    let result = service(index, None, reopened)
        .query(&query("find alpha"))
        .expect("query reopened projection");

    assert_eq!(
        result.candidates[0].relative_path,
        std::path::Path::new("alpha.rs")
    );
}

#[test]
fn persistent_projection_reuses_unchanged_chunks_across_lexical_generations() {
    let workspace = workspace();
    let index = index(&workspace);
    let storage = tempfile::tempdir().expect("storage");
    let path = storage.path().join("semantic.sqlite3");
    let embedded_input_count = Arc::new(AtomicUsize::new(0));
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(CountingEmbedding {
        embedded_input_count: Arc::clone(&embedded_input_count),
    });
    let first_store: Arc<dyn CodebaseVectorStore> = Arc::new(
        SqliteCodebaseVectorStore::open(&CodebaseSemanticStorage::Persistent(path.clone()))
            .expect("open store"),
    );
    let first = CodebaseSemanticService::new(
        Arc::clone(&index),
        model_id(),
        Arc::clone(&embedding),
        first_store,
    )
    .sync()
    .expect("initial sync");
    assert_eq!(first.reused_embedding_count, 0);
    assert_eq!(embedded_input_count.load(Ordering::Relaxed), 2);

    index.rebuild().expect("advance lexical generation");
    let reopened: Arc<dyn CodebaseVectorStore> = Arc::new(
        SqliteCodebaseVectorStore::open(&CodebaseSemanticStorage::Persistent(path))
            .expect("reopen store"),
    );
    let second = CodebaseSemanticService::new(index, model_id(), embedding, reopened)
        .sync()
        .expect("reuse sync");

    assert_eq!(second.reused_embedding_count, 2);
    assert_eq!(embedded_input_count.load(Ordering::Relaxed), 2);
}

#[test]
fn persistent_projection_embeds_only_changed_chunks() {
    let workspace = workspace();
    let index = index(&workspace);
    let storage = tempfile::tempdir().expect("storage");
    let path = storage.path().join("semantic.sqlite3");
    let embedded_input_count = Arc::new(AtomicUsize::new(0));
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(CountingEmbedding {
        embedded_input_count: Arc::clone(&embedded_input_count),
    });
    let store: Arc<dyn CodebaseVectorStore> = Arc::new(
        SqliteCodebaseVectorStore::open(&CodebaseSemanticStorage::Persistent(path))
            .expect("open store"),
    );
    let service = CodebaseSemanticService::new(index.clone(), model_id(), embedding, store);
    service.sync().expect("initial sync");

    std::fs::write(
        workspace.path().join("beta.rs"),
        "pub fn changed_beta_feature() {}\n",
    )
    .expect("change source");
    index.rebuild().expect("advance lexical generation");
    let sync = service.sync().expect("incremental sync");

    assert_eq!(sync.reused_embedding_count, 1);
    assert_eq!(embedded_input_count.load(Ordering::Relaxed), 3);
}

#[test]
fn stale_semantic_generation_is_rejected_after_lexical_refresh() {
    let workspace = workspace();
    let index = index(&workspace);
    let service = service(Arc::clone(&index), None, memory_store());
    service.sync().expect("sync");
    std::fs::write(
        workspace.path().join("beta.rs"),
        "pub fn changed_beta_feature() {}\n",
    )
    .expect("change source");
    index.rebuild().expect("rebuild");

    assert!(matches!(
        service.query(&query("find beta")),
        Err(CodebaseSemanticError::VectorStore(_))
    ));
}

#[test]
fn sync_rejects_model_output_that_does_not_match_workspace_chunks() {
    let workspace = workspace();
    let index = index(&workspace);
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(WrongCardinalityEmbedding);
    let service = CodebaseSemanticService::new(index, model_id(), embedding, memory_store());

    assert!(matches!(
        service.sync(),
        Err(CodebaseSemanticError::InvalidModelResponse(
            "embedding count does not match the synchronized chunk count"
        ))
    ));
}

#[test]
fn cancelled_sync_reuses_completed_batches_on_retry() {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("git marker");
    for index in 0..140 {
        std::fs::write(
            workspace.path().join(format!("file-{index}.rs")),
            format!("pub fn item_{index}() {{}}\n"),
        )
        .expect("source");
    }
    let index = index(&workspace);
    let source = zeta_async_utils::CancellationSource::new();
    let embedding = Arc::new(CancellingBatchEmbedding {
        calls: AtomicUsize::new(0),
        cancellation: Mutex::new(Some(source.clone())),
    });
    let store: Arc<dyn CodebaseVectorStore> = Arc::new(InMemoryCodebaseVectorStore::default());
    let service = CodebaseSemanticService::new(index, model_id(), embedding.clone(), store);

    assert!(matches!(
        service.sync_with_control(&source.token(), None),
        Err(CodebaseSemanticError::Cancelled)
    ));
    *embedding.cancellation.lock().unwrap() = None;
    let retried = service.sync().expect("retry");

    assert_eq!(retried.indexed_chunk_count, 140);
    assert_eq!(retried.reused_embedding_count, 128);
    assert_eq!(embedding.calls.load(Ordering::Relaxed), 2);
}

#[test]
fn transient_embedding_failure_is_retried_and_reported() {
    let workspace = workspace();
    let index = index(&workspace);
    let embedding = Arc::new(TransientOnceEmbedding {
        calls: AtomicUsize::new(0),
    });
    let metrics = Arc::new(RecordingMetrics::default());
    let service =
        CodebaseSemanticService::new(index, model_id(), embedding.clone(), memory_store())
            .with_metrics(metrics.clone());

    let synced = service.sync().expect("retry succeeds");

    assert_eq!(synced.retry_count, 1);
    assert_eq!(embedding.calls.load(Ordering::Relaxed), 2);
    assert!(matches!(
        metrics.metrics.lock().unwrap().as_slice(),
        [CodebaseSemanticMetric::SyncCompleted { retry_count: 1, .. }]
    ));
}

fn service(
    index: Arc<Codebase>,
    rerank: Option<Arc<dyn RerankInvoker>>,
    store: Arc<dyn CodebaseVectorStore>,
) -> CodebaseSemanticService {
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(KeywordEmbedding);
    let service = CodebaseSemanticService::new(index, model_id(), embedding, store);
    match rerank {
        Some(rerank) => service.with_rerank(rerank),
        None => service,
    }
}

fn memory_store() -> Arc<dyn CodebaseVectorStore> {
    Arc::new(InMemoryCodebaseVectorStore::default())
}

fn model_id() -> EmbeddingIndexKey {
    EmbeddingIndexKey::new("keyword-embedding-v1").expect("model id")
}

fn query(text: &str) -> CodebaseSemanticQuery {
    CodebaseSemanticQuery::new(text, NonZeroUsize::new(10).expect("limit")).expect("query")
}

fn index(workspace: &TempDir) -> Arc<Codebase> {
    let index = Arc::new(
        Codebase::open(
            WorkspaceRoot::open(workspace.path()).expect("root"),
            CodebaseStorage::Memory,
            CodebaseLimits::default(),
        )
        .expect("index"),
    );
    index.rebuild().expect("rebuild");
    index
}

fn workspace() -> TempDir {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("git marker");
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
    workspace
}
