use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use tempfile::TempDir;
use zeta_code_index::ChunkReference;
use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexLimits;
use zeta_code_index::CodeIndexOverlayDocument;
use zeta_code_index::CodeIndexStorage;
use zeta_code_index::IndexedLanguage;
use zeta_code_index_cloud::CloudCodeIndexCandidate;
use zeta_code_index_cloud::CloudCodeIndexCapabilities;
use zeta_code_index_cloud::CloudCodeIndexController;
use zeta_code_index_cloud::CloudCodeIndexDeletionSupport;
use zeta_code_index_cloud::CloudCodeIndexDestination;
use zeta_code_index_cloud::CloudCodeIndexGrant;
use zeta_code_index_cloud::CloudCodeIndexGrantId;
use zeta_code_index_cloud::CloudCodeIndexProvider;
use zeta_code_index_cloud::CloudCodeIndexProviderError;
use zeta_code_index_cloud::CloudCodeIndexProviderId;
use zeta_code_index_cloud::CloudCodeIndexProviderRegistry;
use zeta_code_index_cloud::CloudCodeIndexPublication;
use zeta_code_index_cloud::CloudCodeIndexPublicationRequest;
use zeta_code_index_cloud::CloudCodeIndexQueryRequest;
use zeta_code_index_cloud::CloudCodeIndexQueryResult;
use zeta_code_index_cloud::CloudCodeIndexSelection;
use zeta_code_index_cloud::CloudCodeIndexStorage;
use zeta_code_index_semantic::CodeIndexEmbeddingModelId;
use zeta_code_index_semantic::CodeIndexSemanticService;
use zeta_code_index_semantic::CodeIndexVectorStore;
use zeta_code_index_semantic::InMemoryCodeIndexVectorStore;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelProviderError;
use zeta_symbol_index::SymbolIndex;
use zeta_symbol_index::SymbolIndexLimits;
use zeta_symbol_index::SymbolIndexStorage;
use zeta_workspace::WorkspaceRoot;

use crate::CodeRetrievalBudget;
use crate::CodeRetrievalDegradation;
use crate::CodeRetrievalOrigin;
use crate::CodeRetrievalQuery;
use crate::CodeRetrievalService;

struct QueryProvider {
    id: CloudCodeIndexProviderId,
    published: Mutex<Vec<ChunkReference>>,
    fail_query: AtomicBool,
    reverse_query_order: AtomicBool,
}

struct SemanticEmbedding;

impl EmbeddingInvoker for SemanticEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        EmbeddingResponse::new(
            request
                .inputs()
                .iter()
                .map(|input| {
                    if input.contains("semantic_target") || input.contains("meaning query") {
                        EmbeddingVector::new(vec![1.0, 0.0])
                    } else {
                        EmbeddingVector::new(vec![0.0, 1.0])
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    }
}

impl QueryProvider {
    fn new() -> Self {
        Self {
            id: CloudCodeIndexProviderId::new("query-provider").expect("provider id"),
            published: Mutex::new(Vec::new()),
            fail_query: AtomicBool::new(false),
            reverse_query_order: AtomicBool::new(false),
        }
    }
}

impl CloudCodeIndexProvider for QueryProvider {
    fn id(&self) -> &CloudCodeIndexProviderId {
        &self.id
    }

    fn capabilities(&self) -> CloudCodeIndexCapabilities {
        CloudCodeIndexCapabilities {
            deletion: CloudCodeIndexDeletionSupport::IdempotentGrantDeletion,
        }
    }

    fn publish(
        &self,
        request: CloudCodeIndexPublicationRequest,
    ) -> Result<CloudCodeIndexPublication, CloudCodeIndexProviderError> {
        *self.published.lock().expect("published references") = request
            .chunks
            .into_iter()
            .map(|chunk| chunk.reference)
            .collect();
        Ok(CloudCodeIndexPublication {
            remote_generation: format!("projection-{}", request.local_generation),
        })
    }

    fn query(
        &self,
        request: CloudCodeIndexQueryRequest,
    ) -> Result<CloudCodeIndexQueryResult, CloudCodeIndexProviderError> {
        if self.fail_query.load(Ordering::Relaxed) {
            return Err(CloudCodeIndexProviderError::new("query unavailable"));
        }
        let mut references = self.published.lock().expect("published references").clone();
        if self.reverse_query_order.load(Ordering::Relaxed) {
            references.reverse();
        }
        let candidates = references
            .into_iter()
            .take(request.query.result_limit().get())
            .map(|reference| CloudCodeIndexCandidate { reference })
            .collect();
        Ok(CloudCodeIndexQueryResult {
            remote_generation: request.remote_generation,
            candidates,
        })
    }

    fn delete_grant(
        &self,
        _grant: &CloudCodeIndexGrant,
    ) -> Result<(), CloudCodeIndexProviderError> {
        Ok(())
    }
}

#[test]
fn local_retrieval_returns_verified_source_excerpt() {
    let workspace = workspace("pub fn local_recall_marker() {}\n");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let service = CodeRetrievalService::local(index);

    let result = service
        .retrieve(&query("local_recall_marker"))
        .expect("retrieve");

    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].origins, [CodeRetrievalOrigin::LocalLexical]);
    assert!(result.hits[0].content.contains("local_recall_marker"));
    assert!(result.degradations.is_empty());
}

#[test]
fn hybrid_retrieval_fuses_and_deduplicates_the_same_excerpt() {
    let workspace = workspace("pub fn shared_recall_marker() {}\n");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(QueryProvider::new());
    let cloud = ready_cloud(Arc::clone(&index), Arc::clone(&provider));
    let service = CodeRetrievalService::hybrid(index, cloud).expect("hybrid");

    let result = service
        .retrieve(&query("shared_recall_marker"))
        .expect("retrieve");

    assert_eq!(result.hits.len(), 1);
    assert_eq!(
        result.hits[0].origins,
        [
            CodeRetrievalOrigin::LocalLexical,
            CodeRetrievalOrigin::CloudSemantic,
        ]
    );
    assert!(result.degradations.is_empty());
}

#[test]
fn local_semantic_retrieval_adds_dense_candidates_without_cloud() {
    let workspace = workspace("pub fn semantic_target() {}\n");
    std::fs::write(
        workspace.path().join("other.rs"),
        "pub fn unrelated_code() {}\n",
    )
    .expect("other source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let semantic = semantic_service(Arc::clone(&index));
    semantic.sync().expect("semantic sync");
    let service = CodeRetrievalService::local_semantic(index, semantic).expect("local semantic");

    let result = service.retrieve(&query("meaning query")).expect("retrieve");

    assert_eq!(result.hits.len(), 2);
    assert_eq!(
        result.hits[0].reference.relative_path,
        std::path::Path::new("lib.rs")
    );
    assert_eq!(result.hits[0].origins, [CodeRetrievalOrigin::LocalSemantic]);
    assert!(result.degradations.is_empty());
}

#[test]
fn hybrid_retrieval_preserves_cloud_provider_ranking() {
    let workspace = workspace("pub fn first_cloud_candidate() {}\n");
    std::fs::write(
        workspace.path().join("second.rs"),
        "pub fn second_cloud_candidate() {}\n",
    )
    .expect("second source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(QueryProvider::new());
    let cloud = ready_cloud(Arc::clone(&index), Arc::clone(&provider));
    provider.reverse_query_order.store(true, Ordering::Relaxed);
    let mut expected_paths = provider
        .published
        .lock()
        .expect("published references")
        .iter()
        .map(|reference| reference.relative_path.clone())
        .collect::<Vec<_>>();
    expected_paths.reverse();
    let service = CodeRetrievalService::hybrid(index, cloud).expect("hybrid");

    let result = service
        .retrieve(&query("cloud_only_query_without_lexical_hits"))
        .expect("retrieve");
    let actual_paths = result
        .hits
        .iter()
        .map(|hit| hit.reference.relative_path.clone())
        .collect::<Vec<_>>();

    assert_eq!(actual_paths, expected_paths);
    assert!(
        result
            .hits
            .iter()
            .all(|hit| hit.origins == [CodeRetrievalOrigin::CloudSemantic])
    );
}

#[test]
fn cloud_query_failure_falls_back_to_local_results() {
    let workspace = workspace("pub fn fallback_marker() {}\n");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(QueryProvider::new());
    let cloud = ready_cloud(Arc::clone(&index), Arc::clone(&provider));
    provider.fail_query.store(true, Ordering::Relaxed);
    let service = CodeRetrievalService::hybrid(index, cloud).expect("hybrid");

    let result = service
        .retrieve(&query("fallback_marker"))
        .expect("retrieve");

    assert_eq!(result.hits.len(), 1);
    assert_eq!(
        result.degradations,
        [CodeRetrievalDegradation::CloudQueryFailed]
    );
}

#[test]
fn verification_and_content_budgets_discard_unusable_candidates() {
    let workspace = workspace("pub fn budget_marker() {}\n");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let service = CodeRetrievalService::local(Arc::clone(&index)).with_budget(
        CodeRetrievalBudget::default()
            .with_max_item_bytes(NonZeroUsize::new(1).expect("item budget")),
    );
    let budgeted = service
        .retrieve(&query("budget_marker"))
        .expect("budgeted retrieve");
    assert!(budgeted.hits.is_empty());
    assert_eq!(
        budgeted.degradations,
        [CodeRetrievalDegradation::ContentBudgetExceeded { discarded: 1 }]
    );

    std::fs::write(workspace.path().join("lib.rs"), "pub fn changed() {}\n")
        .expect("mutate source");
    let stale = CodeRetrievalService::local(index)
        .retrieve(&query("budget_marker"))
        .expect("stale retrieve");
    assert!(stale.hits.is_empty());
    assert_eq!(
        stale.degradations,
        [CodeRetrievalDegradation::CandidateVerificationFailed { discarded: 1 }]
    );
}

#[test]
fn dirty_overlay_suppresses_old_disk_content_and_materializes_current_text() {
    let workspace = workspace("pub fn persisted_secret() {}\n");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    index
        .synchronize_overlay(CodeIndexOverlayDocument {
            relative_path: "lib.rs".into(),
            editor_revision: 2,
            language: IndexedLanguage::Rust,
            content: "pub fn unsaved_current() {}\n".into(),
        })
        .expect("overlay");
    let service = CodeRetrievalService::local(index);

    assert!(
        service
            .retrieve(&query("persisted_secret"))
            .expect("old query")
            .hits
            .is_empty()
    );
    let current = service
        .retrieve(&query("unsaved_current"))
        .expect("current query");
    assert_eq!(current.hits.len(), 1);
    assert!(current.hits[0].content.contains("unsaved_current"));
}

#[test]
fn symbol_retrieval_returns_the_exact_declaration_with_provenance() {
    let workspace = workspace(
        "const PREFIX: &str = \"outside\";\n\npub fn precise_symbol() {\n    let local = 1;\n}\n",
    );
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let symbols = Arc::new(
        SymbolIndex::open(
            Arc::clone(&index),
            SymbolIndexStorage::Memory,
            SymbolIndexLimits::default(),
        )
        .expect("symbol index"),
    );
    symbols.reconcile().expect("symbol reconcile");
    let service = CodeRetrievalService::local(index)
        .with_symbol_index(symbols)
        .expect("matching symbol root");

    let result = service
        .retrieve(&query("precise_symbol"))
        .expect("retrieve");
    let hit = result
        .hits
        .iter()
        .find(|hit| hit.origins.contains(&CodeRetrievalOrigin::LocalSymbol))
        .expect("symbol hit");

    assert!(hit.content.starts_with("pub fn precise_symbol"));
    assert!(!hit.content.contains("PREFIX"));
    assert_eq!(hit.reference.span.start_line, 2);
    assert!(result.degradations.is_empty());
}

fn workspace(content: &str) -> TempDir {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("git marker");
    std::fs::write(workspace.path().join("lib.rs"), content).expect("source");
    workspace
}

fn index(workspace: &TempDir) -> Arc<CodeIndex> {
    Arc::new(
        CodeIndex::open(
            WorkspaceRoot::open(workspace.path()).expect("root"),
            CodeIndexStorage::Memory,
            CodeIndexLimits::default(),
        )
        .expect("index"),
    )
}

fn ready_cloud(
    index: Arc<CodeIndex>,
    provider: Arc<QueryProvider>,
) -> Arc<CloudCodeIndexController> {
    let provider_trait: Arc<dyn CloudCodeIndexProvider> = provider;
    let providers = CloudCodeIndexProviderRegistry::new([provider_trait]).expect("providers");
    let controller = CloudCodeIndexController::open(
        Arc::clone(&index),
        providers,
        CloudCodeIndexStorage::Memory,
    )
    .expect("cloud controller");
    controller
        .authorize(CloudCodeIndexGrant {
            id: CloudCodeIndexGrantId::new("retrieval-grant").expect("grant id"),
            root_id: index.root_id().as_str().to_owned(),
            destination: CloudCodeIndexDestination::new(
                CloudCodeIndexProviderId::new("query-provider").expect("provider id"),
                "tenant-a",
                "workspace-index",
            )
            .expect("destination"),
            selection: CloudCodeIndexSelection::EntireIndex,
            max_egress_bytes: NonZeroU64::new(1024 * 1024).expect("egress limit"),
        })
        .expect("authorize");
    controller.sync().expect("sync");
    controller
}

fn semantic_service(index: Arc<CodeIndex>) -> Arc<CodeIndexSemanticService> {
    let embedding: Arc<dyn EmbeddingInvoker> = Arc::new(SemanticEmbedding);
    let store: Arc<dyn CodeIndexVectorStore> = Arc::new(InMemoryCodeIndexVectorStore::default());
    Arc::new(CodeIndexSemanticService::new(
        index,
        CodeIndexEmbeddingModelId::new("semantic-test-v1").expect("model id"),
        embedding,
        store,
    ))
}

fn query(text: &str) -> CodeRetrievalQuery {
    CodeRetrievalQuery::new(text, NonZeroUsize::new(10).expect("result limit")).expect("query")
}
