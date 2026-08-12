use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use tempfile::TempDir;
use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexLimits;
use zeta_code_index::CodeIndexStorage;
use zeta_workspace::WorkspaceRoot;

use super::*;

#[derive(Default)]
struct PublicationFacts {
    publication_contents: Vec<Vec<String>>,
    publication_references: Vec<zeta_code_index::ChunkReference>,
}

struct RecordingProvider {
    id: CloudCodeIndexProviderId,
    facts: Mutex<PublicationFacts>,
    delete_calls: AtomicUsize,
    fail_delete: AtomicBool,
    return_unpublished_chunk: AtomicBool,
}

struct UnsafeDeletionProvider {
    id: CloudCodeIndexProviderId,
}

impl UnsafeDeletionProvider {
    fn new() -> Self {
        Self {
            id: CloudCodeIndexProviderId::new("unsafe-deletion").expect("provider id"),
        }
    }
}

impl CloudCodeIndexProvider for UnsafeDeletionProvider {
    fn id(&self) -> &CloudCodeIndexProviderId {
        &self.id
    }

    fn capabilities(&self) -> CloudCodeIndexCapabilities {
        CloudCodeIndexCapabilities {
            deletion: CloudCodeIndexDeletionSupport::Unsupported,
        }
    }

    fn publish(
        &self,
        _request: CloudCodeIndexPublicationRequest,
    ) -> Result<CloudCodeIndexPublication, CloudCodeIndexProviderError> {
        Err(CloudCodeIndexProviderError::new("must not publish"))
    }

    fn query(
        &self,
        _request: CloudCodeIndexQueryRequest,
    ) -> Result<CloudCodeIndexQueryResult, CloudCodeIndexProviderError> {
        Err(CloudCodeIndexProviderError::new("must not query"))
    }

    fn delete_grant(
        &self,
        _grant: &CloudCodeIndexGrant,
    ) -> Result<(), CloudCodeIndexProviderError> {
        Err(CloudCodeIndexProviderError::new("deletion unsupported"))
    }
}

impl RecordingProvider {
    fn new() -> Self {
        Self {
            id: CloudCodeIndexProviderId::new("recording").expect("provider id"),
            facts: Mutex::new(PublicationFacts::default()),
            delete_calls: AtomicUsize::new(0),
            fail_delete: AtomicBool::new(false),
            return_unpublished_chunk: AtomicBool::new(false),
        }
    }
}

impl CloudCodeIndexProvider for RecordingProvider {
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
        let mut facts = self.facts.lock().expect("facts");
        facts.publication_references = request
            .chunks
            .iter()
            .map(|chunk| chunk.reference.clone())
            .collect();
        facts.publication_contents.push(
            request
                .chunks
                .into_iter()
                .map(|chunk| chunk.content)
                .collect(),
        );
        Ok(CloudCodeIndexPublication {
            remote_generation: format!("workspace-projection-{}", request.local_generation),
        })
    }

    fn query(
        &self,
        request: CloudCodeIndexQueryRequest,
    ) -> Result<CloudCodeIndexQueryResult, CloudCodeIndexProviderError> {
        let mut references = self
            .facts
            .lock()
            .expect("facts")
            .publication_references
            .clone();
        if self.return_unpublished_chunk.load(Ordering::Relaxed) {
            let reference = references.first_mut().expect("published reference");
            reference.key = zeta_code_index::ChunkKey::parse(format!("sha256:{}", "0".repeat(64)))
                .expect("synthetic chunk key");
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
        self.delete_calls.fetch_add(1, Ordering::Relaxed);
        if self.fail_delete.load(Ordering::Relaxed) {
            Err(CloudCodeIndexProviderError::new("delete unavailable"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn workspace_chunks_are_the_only_cloud_publication_unit() {
    let workspace = workspace();
    std::fs::create_dir(workspace.path().join("src")).expect("src");
    std::fs::write(
        workspace.path().join("src/lib.rs"),
        "pub fn selected_source() -> bool { true }\n",
    )
    .expect("source");
    std::fs::write(
        workspace.path().join("README.md"),
        "outside_selection_marker\n",
    )
    .expect("readme");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(RecordingProvider::new());
    let controller = controller(Arc::clone(&index), Arc::clone(&provider));
    let selection = CloudCodeIndexSelection::path_prefixes(vec!["src".into()]).expect("scope");
    let ceiling = NonZeroU64::new(1024 * 1024).expect("ceiling");

    let preview = controller.preview(&selection, ceiling).expect("preview");
    assert_eq!(preview.file_count, 1);
    assert_eq!(preview.upload_unit_count, preview.chunk_count);

    controller
        .authorize(grant(&index, "cloud-grant", selection.clone(), ceiling))
        .expect("authorize cloud projection");
    let cloud = controller.sync().expect("sync cloud projection");
    assert_eq!(cloud.deployment_mode, CodeIndexDeploymentMode::Cloud);
    assert_eq!(cloud.state, CloudCodeIndexState::Ready);
    let queried = controller
        .query(
            &CloudCodeIndexQuery::new(
                "selected_source",
                NonZeroUsize::new(10).expect("result limit"),
            )
            .expect("query"),
        )
        .expect("cloud query");
    assert!(!queried.candidates.is_empty());
    {
        let facts = provider.facts.lock().expect("facts");
        assert_eq!(facts.publication_contents.len(), 1);
        assert!(facts.publication_contents[0][0].contains("selected_source"));
        assert!(
            facts.publication_contents[0]
                .iter()
                .all(|content| !content.contains("outside_selection_marker"))
        );
    }

    let local = controller.revoke().expect("revoke cloud projection");
    assert_eq!(local.deployment_mode, CodeIndexDeploymentMode::LocalOnly);
    assert_eq!(local.state, CloudCodeIndexState::LocalOnly);
}

#[test]
fn cloud_query_rejects_a_chunk_not_published_by_the_workspace() {
    let workspace = workspace();
    std::fs::write(
        workspace.path().join("lib.rs"),
        "pub fn workspace_owned_chunk() {}\n",
    )
    .expect("source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(RecordingProvider::new());
    let controller = controller(Arc::clone(&index), Arc::clone(&provider));
    controller
        .authorize(grant(
            &index,
            "exact-chunk-grant",
            CloudCodeIndexSelection::EntireIndex,
            NonZeroU64::new(1024).expect("limit"),
        ))
        .expect("authorize");
    controller.sync().expect("sync");
    provider
        .return_unpublished_chunk
        .store(true, Ordering::Relaxed);

    assert!(matches!(
        controller.query(
            &CloudCodeIndexQuery::new(
                "workspace_owned_chunk",
                NonZeroUsize::new(10).expect("result limit"),
            )
            .expect("query"),
        ),
        Err(CloudCodeIndexError::InvalidProviderResult(
            "candidate is not an exact chunk from the published Workspace generation"
        ))
    ));
}

#[test]
fn legacy_managed_grant_is_migrated_to_deletion_only_state() {
    let workspace = workspace();
    std::fs::write(workspace.path().join("lib.rs"), "pub fn legacy() {}\n").expect("source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(RecordingProvider::new());
    let state = tempfile::tempdir().expect("state");
    let database = state.path().join("cloud.sqlite3");
    write_legacy_state(&database, &index, "Managed");
    let controller = CloudCodeIndexController::open(
        Arc::clone(&index),
        registry(Arc::clone(&provider)),
        CloudCodeIndexStorage::Persistent(database),
    )
    .expect("migrate legacy state");

    let status = controller.status().expect("status");
    assert_eq!(status.state, CloudCodeIndexState::Revoking);
    assert!(matches!(
        controller.sync(),
        Err(CloudCodeIndexError::InvalidState)
    ));
    assert_eq!(
        controller.revoke().expect("delete legacy grant").state,
        CloudCodeIndexState::LocalOnly
    );
    assert_eq!(provider.delete_calls.load(Ordering::Relaxed), 1);
}

#[test]
fn legacy_projection_grant_preserves_its_ready_generation() {
    let workspace = workspace();
    std::fs::write(workspace.path().join("lib.rs"), "pub fn legacy() {}\n").expect("source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(RecordingProvider::new());
    let state = tempfile::tempdir().expect("state");
    let database = state.path().join("cloud.sqlite3");
    write_legacy_state(&database, &index, "Projection");
    let controller = CloudCodeIndexController::open(
        index,
        registry(provider),
        CloudCodeIndexStorage::Persistent(database),
    )
    .expect("migrate legacy state");

    let status = controller.status().expect("status");
    assert_eq!(status.state, CloudCodeIndexState::Ready);
    assert_eq!(status.remote_generation.as_deref(), Some("legacy-remote-1"));
}

#[test]
fn cloud_preview_requires_a_published_local_generation() {
    let workspace = workspace();
    std::fs::write(workspace.path().join("lib.rs"), "pub fn not_ready() {}\n").expect("source");
    let index = index(&workspace);
    let provider = Arc::new(RecordingProvider::new());
    let controller = controller(index, provider);

    assert!(matches!(
        controller.preview(
            &CloudCodeIndexSelection::EntireIndex,
            NonZeroU64::new(1024).expect("limit"),
        ),
        Err(CloudCodeIndexError::LocalIndexNotReady)
    ));
}

#[test]
fn consent_is_bounded_and_cannot_be_silently_widened() {
    let workspace = workspace();
    std::fs::write(workspace.path().join("lib.rs"), "pub fn bounded() {}\n").expect("source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(RecordingProvider::new());
    let controller = controller(Arc::clone(&index), provider);
    let too_small = NonZeroU64::new(1).expect("limit");
    let preview = controller
        .preview(&CloudCodeIndexSelection::EntireIndex, too_small)
        .expect("preview");
    assert_eq!(preview.limit, CloudCodeIndexLimitDisposition::ExceedsLimit);
    assert!(matches!(
        controller.authorize(grant(
            &index,
            "too-small",
            CloudCodeIndexSelection::EntireIndex,
            too_small,
        )),
        Err(CloudCodeIndexError::EgressLimitExceeded)
    ));

    let ceiling = NonZeroU64::new(1024).expect("limit");
    controller
        .authorize(grant(
            &index,
            "first",
            CloudCodeIndexSelection::EntireIndex,
            ceiling,
        ))
        .expect("first grant");
    assert!(matches!(
        controller.authorize(grant(
            &index,
            "widened",
            CloudCodeIndexSelection::EntireIndex,
            ceiling,
        )),
        Err(CloudCodeIndexError::ConsentConflict)
    ));
}

#[test]
fn grant_is_rejected_without_idempotent_provider_deletion() {
    let workspace = workspace();
    std::fs::write(workspace.path().join("lib.rs"), "pub fn protected() {}\n").expect("source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider: Arc<dyn CloudCodeIndexProvider> = Arc::new(UnsafeDeletionProvider::new());
    let registry = CloudCodeIndexProviderRegistry::new([provider]).expect("registry");
    let controller =
        CloudCodeIndexController::open(Arc::clone(&index), registry, CloudCodeIndexStorage::Memory)
            .expect("controller");
    let grant = CloudCodeIndexGrant {
        id: CloudCodeIndexGrantId::new("unsafe-grant").expect("grant"),
        root_id: index.root_id().as_str().to_owned(),
        destination: CloudCodeIndexDestination::new(
            CloudCodeIndexProviderId::new("unsafe-deletion").expect("provider"),
            "tenant-a",
            "code-index",
        )
        .expect("destination"),
        selection: CloudCodeIndexSelection::EntireIndex,
        max_egress_bytes: NonZeroU64::new(1024).expect("limit"),
    };

    assert!(matches!(
        controller.authorize(grant),
        Err(CloudCodeIndexError::DeletionUnsupported)
    ));
    assert_eq!(
        controller.status().expect("status").state,
        CloudCodeIndexState::LocalOnly
    );
}

#[test]
fn failed_deletion_remains_durable_and_can_resume_after_restart() {
    let workspace = workspace();
    std::fs::write(
        workspace.path().join("lib.rs"),
        "pub fn durable_delete() {}\n",
    )
    .expect("source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(RecordingProvider::new());
    let registry = registry(Arc::clone(&provider));
    let state = tempfile::tempdir().expect("state");
    let storage = CloudCodeIndexStorage::Persistent(state.path().join("cloud.sqlite3"));
    let controller =
        CloudCodeIndexController::open(Arc::clone(&index), registry.clone(), storage.clone())
            .expect("controller");
    controller
        .authorize(grant(
            &index,
            "delete-grant",
            CloudCodeIndexSelection::EntireIndex,
            NonZeroU64::new(1024).expect("limit"),
        ))
        .expect("authorize");
    controller.sync().expect("sync");
    provider.fail_delete.store(true, Ordering::Relaxed);
    assert!(matches!(
        controller.revoke(),
        Err(CloudCodeIndexError::Provider(_))
    ));
    assert_eq!(
        controller.status().expect("status").state,
        CloudCodeIndexState::Revoking
    );
    drop(controller);

    provider.fail_delete.store(false, Ordering::Relaxed);
    let reopened = CloudCodeIndexController::open(index, registry, storage).expect("reopen");
    let status = reopened.revoke().expect("retry delete");
    assert_eq!(status.state, CloudCodeIndexState::LocalOnly);
    assert_eq!(provider.delete_calls.load(Ordering::Relaxed), 2);
}

#[test]
fn a_new_local_generation_marks_the_cloud_projection_stale() {
    let workspace = workspace();
    let source_path = workspace.path().join("lib.rs");
    std::fs::write(&source_path, "pub fn before() {}\n").expect("source");
    let index = index(&workspace);
    index.rebuild().expect("rebuild");
    let provider = Arc::new(RecordingProvider::new());
    let controller = controller(Arc::clone(&index), provider);
    controller
        .authorize(grant(
            &index,
            "stale-grant",
            CloudCodeIndexSelection::EntireIndex,
            NonZeroU64::new(1024).expect("limit"),
        ))
        .expect("authorize");
    controller.sync().expect("sync");

    std::fs::write(&source_path, "pub fn after() {}\n").expect("change");
    index
        .refresh_observed_paths(std::slice::from_ref(&source_path))
        .expect("refresh");

    assert_eq!(
        controller.status().expect("status").state,
        CloudCodeIndexState::Stale
    );
}

fn workspace() -> TempDir {
    let directory = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(directory.path().join(".git")).expect("git marker");
    directory
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

fn controller(
    index: Arc<CodeIndex>,
    provider: Arc<RecordingProvider>,
) -> Arc<CloudCodeIndexController> {
    CloudCodeIndexController::open(index, registry(provider), CloudCodeIndexStorage::Memory)
        .expect("controller")
}

fn registry(provider: Arc<RecordingProvider>) -> CloudCodeIndexProviderRegistry {
    let provider: Arc<dyn CloudCodeIndexProvider> = provider;
    CloudCodeIndexProviderRegistry::new([provider]).expect("registry")
}

fn grant(
    index: &CodeIndex,
    id: &str,
    selection: CloudCodeIndexSelection,
    max_egress_bytes: NonZeroU64,
) -> CloudCodeIndexGrant {
    CloudCodeIndexGrant {
        id: CloudCodeIndexGrantId::new(id).expect("grant id"),
        root_id: index.root_id().as_str().to_owned(),
        destination: CloudCodeIndexDestination::new(
            CloudCodeIndexProviderId::new("recording").expect("provider"),
            "tenant-a",
            "code-index",
        )
        .expect("destination"),
        selection,
        max_egress_bytes,
    }
}

fn write_legacy_state(database: &std::path::Path, index: &CodeIndex, mode: &str) {
    let connection = rusqlite::Connection::open(database).expect("open legacy state");
    connection
        .execute_batch(
            "CREATE TABLE cloud_code_index_metadata (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )
        .expect("legacy schema");
    let state = serde_json::json!({
        "phase": "Ready",
        "grant": {
            "id": "legacy-grant",
            "root_id": index.root_id().as_str(),
            "mode": mode,
            "destination": {
                "provider": "recording",
                "tenant": "tenant-a",
                "collection": "code-index"
            },
            "selection": "EntireIndex",
            "max_egress_bytes": 1024
        },
        "synced_local_generation": index.snapshot().expect("snapshot").generation,
        "remote_generation": "legacy-remote-1"
    });
    for (key, value) in [
        ("root_id", index.root_id().as_str().to_owned()),
        ("schema_version", "1".to_owned()),
        ("state", state.to_string()),
    ] {
        connection
            .execute(
                "INSERT INTO cloud_code_index_metadata(key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .expect("legacy metadata");
    }
}
