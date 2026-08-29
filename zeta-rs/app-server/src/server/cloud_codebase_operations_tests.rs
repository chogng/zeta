use std::sync::Arc;
use std::sync::Mutex;

use zeta_cloud_codebase::CloudCodebaseCandidate;
use zeta_cloud_codebase::CloudCodebaseCapabilities;
use zeta_cloud_codebase::CloudCodebaseDeletionSupport;
use zeta_cloud_codebase::CloudCodebaseGrant;
use zeta_cloud_codebase::CloudCodebaseProvider;
use zeta_cloud_codebase::CloudCodebaseProviderError;
use zeta_cloud_codebase::CloudCodebaseProviderId;
use zeta_cloud_codebase::CloudCodebaseProviderRegistry;
use zeta_cloud_codebase::CloudCodebasePublication;
use zeta_cloud_codebase::CloudCodebasePublicationRequest;
use zeta_cloud_codebase::CloudCodebaseQueryRequest;
use zeta_cloud_codebase::CloudCodebaseQueryResult;
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
    id: CloudCodebaseProviderId,
    publications: Mutex<usize>,
    candidates: Mutex<Vec<zeta_codebase::ChunkReference>>,
    deletions: Mutex<usize>,
}

impl RecordingProvider {
    fn new() -> Self {
        Self {
            id: CloudCodebaseProviderId::new("recording").expect("provider id"),
            publications: Mutex::new(0),
            candidates: Mutex::new(Vec::new()),
            deletions: Mutex::new(0),
        }
    }
}

impl CloudCodebaseProvider for RecordingProvider {
    fn id(&self) -> &CloudCodebaseProviderId {
        &self.id
    }

    fn capabilities(&self) -> CloudCodebaseCapabilities {
        CloudCodebaseCapabilities {
            deletion: CloudCodebaseDeletionSupport::IdempotentGrantDeletion,
        }
    }

    fn publish(
        &self,
        request: CloudCodebasePublicationRequest,
    ) -> Result<CloudCodebasePublication, CloudCodebaseProviderError> {
        assert!(!request.chunks.is_empty());
        *self.candidates.lock().expect("candidates") = request
            .chunks
            .iter()
            .map(|chunk| chunk.reference.clone())
            .collect();
        *self.publications.lock().expect("publication count") += 1;
        Ok(CloudCodebasePublication {
            remote_generation: format!("workspace-projection-{}", request.local_generation),
        })
    }

    fn query(
        &self,
        request: CloudCodebaseQueryRequest,
    ) -> Result<CloudCodebaseQueryResult, CloudCodebaseProviderError> {
        let candidates = self
            .candidates
            .lock()
            .expect("candidates")
            .iter()
            .take(request.query.result_limit().get())
            .cloned()
            .map(|reference| CloudCodebaseCandidate { reference })
            .collect();
        Ok(CloudCodebaseQueryResult {
            remote_generation: request.remote_generation,
            candidates,
        })
    }

    fn delete_grant(&self, _grant: &CloudCodebaseGrant) -> Result<(), CloudCodebaseProviderError> {
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
    let provider_trait: Arc<dyn CloudCodebaseProvider> = provider.clone();
    let providers = CloudCodebaseProviderRegistry::new([provider_trait]).expect("providers");
    let server = server()
        .with_cloud_codebase_providers(providers)
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
    let Ok(local_index) = server.codebase_service() else {
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
    assert_eq!(initialize["result"]["capabilities"]["cloudCodebase"], true);

    let status = call(
        &server,
        &mut connection,
        2,
        "workspace/codebase/cloud/status",
        serde_json::json!({}),
    );
    assert_eq!(status["result"]["deploymentMode"], "localOnly");

    let preview = call(
        &server,
        &mut connection,
        3,
        "workspace/codebase/cloud/preview",
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
        "workspace/codebase/cloud/preview",
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
        "workspace/codebase/cloud/authorize",
        serde_json::json!({"grant": cloud_grant}),
    );
    assert_eq!(authorized["result"]["state"], "granted");

    let legacy_grant = call(
        &server,
        &mut connection,
        51,
        "workspace/codebase/cloud/authorize",
        serde_json::json!({"grant": {
            "grantId": "legacy-managed-grant",
            "codebaseId": "legacy-managed-codebase",
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
        "workspace/codebase/cloud/sync",
        serde_json::json!({}),
    );
    assert_eq!(synced["result"]["deploymentMode"], "cloud");
    assert_eq!(synced["result"]["state"], "ready");

    let retrieval = call(
        &server,
        &mut connection,
        7,
        "workspace/codebase/retrieve",
        serde_json::json!({"query": "cloud_mode_selection", "maxResults": 10}),
    );
    assert!(retrieval["result"]["hits"][0].get("origins").is_none());
    assert_eq!(retrieval["result"]["degradations"], serde_json::json!([]));

    let conflict = call(
        &server,
        &mut connection,
        8,
        "workspace/codebase/cloud/authorize",
        serde_json::json!({"grant": grant_json("conflicting-grant")}),
    );
    assert_eq!(conflict["error"]["message"], "CloudCodebaseConsentConflict");

    let revoked = call(
        &server,
        &mut connection,
        9,
        "workspace/codebase/cloud/revoke",
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
    let provider: Arc<dyn CloudCodebaseProvider> = Arc::new(RecordingProvider::new());
    let providers = CloudCodebaseProviderRegistry::new([provider]).expect("providers");
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

    assert_eq!(initialize["result"]["capabilities"]["cloudCodebase"], true);
    assert!(server.cloud_codebase_service().is_ok());
}

fn grant_json(id: &str) -> serde_json::Value {
    serde_json::json!({
        "grantId": id,
        "codebaseId": format!("codebase-{id}"),
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
