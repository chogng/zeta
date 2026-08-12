use std::sync::Arc;
use std::sync::Mutex;

use zeta_code_index_cloud::CloudCodeIndexCandidate;
use zeta_code_index_cloud::CloudCodeIndexCapabilities;
use zeta_code_index_cloud::CloudCodeIndexDeletionSupport;
use zeta_code_index_cloud::CloudCodeIndexGrant;
use zeta_code_index_cloud::CloudCodeIndexProvider;
use zeta_code_index_cloud::CloudCodeIndexProviderError;
use zeta_code_index_cloud::CloudCodeIndexProviderId;
use zeta_code_index_cloud::CloudCodeIndexProviderRegistry;
use zeta_code_index_cloud::CloudCodeIndexPublication;
use zeta_code_index_cloud::CloudCodeIndexPublicationRequest;
use zeta_code_index_cloud::CloudCodeIndexQueryRequest;
use zeta_code_index_cloud::CloudCodeIndexQueryResult;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;
use zeta_workspace::WorkspaceTrustSource;

use crate::local::ProviderModelService;
use crate::server::WorkspaceSwitchTrustPolicy;

use super::*;

struct RecordingProvider {
    id: CloudCodeIndexProviderId,
    publications: Mutex<usize>,
    candidates: Mutex<Vec<zeta_code_index::ChunkReference>>,
    deletions: Mutex<usize>,
}

impl RecordingProvider {
    fn new() -> Self {
        Self {
            id: CloudCodeIndexProviderId::new("recording").expect("provider id"),
            publications: Mutex::new(0),
            candidates: Mutex::new(Vec::new()),
            deletions: Mutex::new(0),
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
        assert!(!request.chunks.is_empty());
        *self.candidates.lock().expect("candidates") = request
            .chunks
            .iter()
            .map(|chunk| chunk.reference.clone())
            .collect();
        *self.publications.lock().expect("publication count") += 1;
        Ok(CloudCodeIndexPublication {
            remote_generation: format!("workspace-projection-{}", request.local_generation),
        })
    }

    fn query(
        &self,
        request: CloudCodeIndexQueryRequest,
    ) -> Result<CloudCodeIndexQueryResult, CloudCodeIndexProviderError> {
        let candidates = self
            .candidates
            .lock()
            .expect("candidates")
            .iter()
            .take(request.query.result_limit().get())
            .cloned()
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
        *self.deletions.lock().expect("deletion count") += 1;
        Ok(())
    }
}

#[test]
fn rpc_exposes_workspace_projection_preview_consent_sync_and_revoke() {
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::create_dir(workspace.path().join(".git")).expect("git marker");
    std::fs::write(
        workspace.path().join("lib.rs"),
        "pub fn cloud_mode_selection() -> bool { true }\n",
    )
    .expect("source");
    let provider = Arc::new(RecordingProvider::new());
    let provider_trait: Arc<dyn CloudCodeIndexProvider> = provider.clone();
    let providers = CloudCodeIndexProviderRegistry::new([provider_trait]).expect("providers");
    let server = server()
        .with_cloud_code_index_providers(providers)
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(
                WorkspaceTrustSource::HostConfiguration,
            ),
        )
        .expect("host");
    server
        .switch_local_workspace_root(workspace.path().to_path_buf())
        .expect("switch");
    let Ok(local_index) = server.code_index_service() else {
        panic!("local index should be installed");
    };
    local_index.rebuild().expect("rebuild");
    let mut connection = server.connection();
    let initialize = call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );
    assert_eq!(initialize["result"]["capabilities"]["cloudCodeIndex"], true);

    let status = call(
        &server,
        &mut connection,
        2,
        "workspace/codeIndex/cloud/status",
        serde_json::json!({}),
    );
    assert_eq!(status["result"]["deploymentMode"], "localOnly");

    let preview = call(
        &server,
        &mut connection,
        3,
        "workspace/codeIndex/cloud/preview",
        serde_json::json!({
            "selection": {"type": "entireIndex"},
            "maxEgressBytes": 1048576
        }),
    );
    assert_eq!(preview["result"]["withinLimit"], true);
    assert_eq!(preview["result"]["fileCount"], 1);
    assert_eq!(
        preview["result"]["uploadUnitCount"],
        preview["result"]["chunkCount"]
    );

    let legacy_preview = call(
        &server,
        &mut connection,
        4,
        "workspace/codeIndex/cloud/preview",
        serde_json::json!({
            "mode": "managed",
            "selection": {"type": "entireIndex"},
            "maxEgressBytes": 1048576
        }),
    );
    assert_eq!(legacy_preview["error"]["message"], "InvalidParams");

    let cloud_grant = grant_json("cloud-grant");
    let authorized = call(
        &server,
        &mut connection,
        5,
        "workspace/codeIndex/cloud/authorize",
        serde_json::json!({"grant": cloud_grant}),
    );
    assert_eq!(authorized["result"]["state"], "granted");

    let legacy_grant = call(
        &server,
        &mut connection,
        51,
        "workspace/codeIndex/cloud/authorize",
        serde_json::json!({"grant": {
            "grantId": "legacy-managed-grant",
            "mode": "managed",
            "destination": {
                "provider": "recording",
                "tenant": "tenant-a",
                "collection": "workspace-index"
            },
            "selection": {"type": "entireIndex"},
            "maxEgressBytes": 1048576
        }}),
    );
    assert_eq!(legacy_grant["error"]["message"], "InvalidParams");
    let synced = call(
        &server,
        &mut connection,
        6,
        "workspace/codeIndex/cloud/sync",
        serde_json::json!({}),
    );
    assert_eq!(synced["result"]["deploymentMode"], "cloud");
    assert_eq!(synced["result"]["state"], "ready");

    let retrieval = call(
        &server,
        &mut connection,
        7,
        "workspace/codeIndex/retrieve",
        serde_json::json!({"query": "cloud_mode_selection", "maxResults": 10}),
    );
    assert_eq!(
        retrieval["result"]["hits"][0]["origins"],
        serde_json::json!(["localLexical", "cloudSemantic"])
    );
    assert_eq!(retrieval["result"]["degradations"], serde_json::json!([]));

    let conflict = call(
        &server,
        &mut connection,
        8,
        "workspace/codeIndex/cloud/authorize",
        serde_json::json!({"grant": grant_json("conflicting-grant")}),
    );
    assert_eq!(
        conflict["error"]["message"],
        "CloudCodeIndexConsentConflict"
    );

    let revoked = call(
        &server,
        &mut connection,
        9,
        "workspace/codeIndex/cloud/revoke",
        serde_json::json!({}),
    );
    assert_eq!(revoked["result"]["deploymentMode"], "localOnly");

    assert_eq!(*provider.publications.lock().unwrap(), 1);
    assert_eq!(*provider.deletions.lock().unwrap(), 1);
}

#[test]
fn standard_local_composition_can_install_cloud_provider_registry_before_activation() {
    let profile = tempfile::tempdir().expect("profile");
    let workspace = tempfile::tempdir().expect("workspace");
    std::fs::write(workspace.path().join("lib.rs"), "pub fn indexed() {}\n").expect("source");
    let provider: Arc<dyn CloudCodeIndexProvider> = Arc::new(RecordingProvider::new());
    let providers = CloudCodeIndexProviderRegistry::new([provider]).expect("providers");
    let options = crate::LocalAppServerOptions::new(profile.path())
        .with_workspace_root(workspace.path())
        .without_built_in_skills();
    let server = crate::open_local_app_server_with_cloud_providers(options, providers)
        .expect("local server");
    let mut connection = server.connection();

    let initialize = call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );

    assert_eq!(initialize["result"]["capabilities"]["cloudCodeIndex"], true);
    assert!(server.cloud_code_index_service().is_ok());
}

fn grant_json(id: &str) -> serde_json::Value {
    serde_json::json!({
        "grantId": id,
        "destination": {
            "provider": "recording",
            "tenant": "tenant-a",
            "collection": "workspace-index"
        },
        "selection": {"type": "entireIndex"},
        "maxEgressBytes": 1048576
    })
}

fn server() -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    AppServer::new(
        sessions,
        Arc::new(ProviderModelService::new(Arc::new(EchoModel))),
    )
}

fn call(
    server: &AppServer,
    connection: &mut super::super::ConnectionState,
    id: u64,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    serde_json::from_str(
        &server.handle_json(
            connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            })
            .to_string(),
        ),
    )
    .expect("response")
}
