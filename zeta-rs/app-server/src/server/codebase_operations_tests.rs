use super::*;
use crate::CodebaseModels;
use crate::local::ProviderModelService;
use crate::server::WorkspaceSwitchTrustPolicy;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use zeta_config::AgentGrepBackend;
use zeta_config::ConfigStore;
use zeta_config::ResolvedConfig;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingRuntimeIdentity;
use zeta_model_provider::EmbeddingRuntimeRequest;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider::RerankInvoker;
use zeta_model_provider::RerankRuntimeRequest;
use zeta_model_provider::SemanticModelProvider;
use zeta_model_provider::SemanticRuntimeLocation;
use zeta_state::WorkspaceIndexKind;
use zeta_workspace::WorkspaceTrustSource;

struct SemanticTestEmbedding;

impl EmbeddingInvoker for SemanticTestEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        EmbeddingResponse::new(
            request
                .inputs()
                .iter()
                .map(|input| {
                    let values = if input.contains("conceptual execution flow")
                        || input.contains("hidden_semantic_target")
                    {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    };
                    EmbeddingVector::new(values)
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    }
}

struct SemanticTestProvider {
    embedding_runtime_count: AtomicUsize,
}

impl SemanticModelProvider for SemanticTestProvider {
    fn embedding_runtime_identity(
        &self,
        request: &EmbeddingRuntimeRequest,
    ) -> Result<EmbeddingRuntimeIdentity, ModelProviderError> {
        EmbeddingRuntimeIdentity::new(format!(
            "test:{}:{}",
            request.model.provider, request.model.model
        ))
    }

    fn embedding_runtime_location(
        &self,
        _: &EmbeddingRuntimeRequest,
    ) -> Result<SemanticRuntimeLocation, ModelProviderError> {
        Ok(SemanticRuntimeLocation::Device)
    }

    fn rerank_runtime_location(
        &self,
        _: &RerankRuntimeRequest,
    ) -> Result<SemanticRuntimeLocation, ModelProviderError> {
        Ok(SemanticRuntimeLocation::Device)
    }

    fn embedding_runtime(
        &self,
        _: EmbeddingRuntimeRequest,
    ) -> Result<Arc<dyn EmbeddingInvoker>, ModelProviderError> {
        self.embedding_runtime_count.fetch_add(1, Ordering::Relaxed);
        Ok(Arc::new(SemanticTestEmbedding))
    }

    fn rerank_runtime(
        &self,
        _: RerankRuntimeRequest,
    ) -> Result<Arc<dyn RerankInvoker>, ModelProviderError> {
        Err(ModelProviderError::Unavailable(
            "test rerank is not configured".into(),
        ))
    }
}

#[test]
fn rpc_reports_generation_and_returns_revision_bound_local_chunks() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join(".git")).unwrap();
    std::fs::write(
        workspace.path().join("lib.rs"),
        "pub fn workspace_side_chunking() -> bool { true }\n",
    )
    .unwrap();
    let server = server()
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(
                WorkspaceTrustSource::HostConfiguration,
            ),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path().to_path_buf())
        .unwrap();
    let Ok(codebase) = server.codebase_service() else {
        panic!("Codebase should be installed");
    };
    codebase.rebuild().unwrap();
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
    assert_eq!(initialize["result"]["capabilities"]["codebase"], true);

    let status = call(
        &server,
        &mut connection,
        2,
        "workspace/codebase/status",
        serde_json::json!({}),
    );
    assert_eq!(status["result"]["state"], "ready");
    assert!(status["result"]["generation"].as_u64().unwrap() >= 1);
    assert!(status["result"].get("semantic").is_none());

    let search = call(
        &server,
        &mut connection,
        3,
        "workspace/codebase/search",
        serde_json::json!({"query": "workspace_side_chunking", "maxResults": 10}),
    );
    assert_eq!(search["result"]["hits"][0]["path"], "lib.rs");
    assert_eq!(search["result"]["hits"][0]["language"], "rust");
    assert!(
        search["result"]["hits"][0]["sourceRevision"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(
        search["result"]["hits"][0]["content"]
            .as_str()
            .unwrap()
            .contains("workspace_side_chunking")
    );

    let retrieval = call(
        &server,
        &mut connection,
        4,
        "workspace/codebase/retrieve",
        serde_json::json!({"query": "workspace_side_chunking", "maxResults": 10}),
    );
    assert_eq!(retrieval["result"]["hits"][0]["path"], "lib.rs");
    assert!(retrieval["result"]["hits"][0].get("origins").is_none());
    assert_eq!(retrieval["result"]["degradations"], serde_json::json!([]));

    let invalid_limit = call(
        &server,
        &mut connection,
        5,
        "workspace/codebase/search",
        serde_json::json!({"query": "workspace_side_chunking", "maxResults": 101}),
    );
    assert_eq!(invalid_limit["error"]["message"], "InvalidParams");

    let invalid_empty_params = call(
        &server,
        &mut connection,
        6,
        "workspace/codebase/status",
        serde_json::json!({"unexpected": true}),
    );
    assert_eq!(invalid_empty_params["error"]["message"], "InvalidParams");
}

#[test]
fn fast_regex_rpc_rebuilds_then_disables_and_deletes_the_project_index() {
    let workspace = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join(".git")).unwrap();
    std::fs::write(
        workspace.path().join("source.rs"),
        "fast_regex_rpc_marker\n",
    )
    .unwrap();
    let config = Arc::new(ConfigStore::open(profile.path().join("config.sqlite3")).unwrap());
    let index_storage = Arc::new(zeta_state::StateRuntime::open(profile.path()).unwrap());
    let resolved = ResolvedConfig {
        agent_grep_backend: AgentGrepBackend::FastRegex,
        ..ResolvedConfig::default()
    };
    let server = server()
        .with_config_store(config)
        .with_state_runtime(Arc::clone(&index_storage))
        .with_local_tool_config(crate::local_tools::LocalToolConfig::from_resolved(
            &resolved,
        ))
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(
                WorkspaceTrustSource::HostConfiguration,
            ),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path().to_path_buf())
        .unwrap();
    let workspace_id = server.active_workspace_trust_id().unwrap();
    let index_directory =
        index_storage.index_directory(&workspace_id, WorkspaceIndexKind::AgentGrep);
    let mut connection = server.connection();
    call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );
    let enabled = call(
        &server,
        &mut connection,
        2,
        "config/update",
        serde_json::json!({
            "commandId": "enable-fast-regex",
            "expectedRevision": 0,
            "agentGrepBackend": "fastRegex"
        }),
    );
    assert_eq!(enabled["result"]["revision"], 1);

    let initial = call(
        &server,
        &mut connection,
        3,
        "workspace/agentGrep/fastRegex/status",
        serde_json::json!({}),
    );
    assert_eq!(initial["result"]["enabled"], true);
    assert_eq!(initial["result"]["active"], false);

    let rebuilt = call(
        &server,
        &mut connection,
        4,
        "workspace/agentGrep/fastRegex/rebuild",
        serde_json::json!({}),
    );
    assert_eq!(rebuilt["result"]["active"], true);
    assert!(rebuilt["result"]["generation"].as_u64().unwrap() >= 1);
    assert!(index_directory.join("manifests").is_dir());

    let deleted = call(
        &server,
        &mut connection,
        5,
        "workspace/agentGrep/fastRegex/disableAndDelete",
        serde_json::json!({
            "commandId": "disable-delete-fast-regex",
            "expectedRevision": 1
        }),
    );
    assert_eq!(deleted["result"]["config"]["revision"], 2);
    assert_eq!(deleted["result"]["deletion"], "cleared");
    assert!(!index_directory.exists());

    let disabled = call(
        &server,
        &mut connection,
        6,
        "workspace/agentGrep/fastRegex/status",
        serde_json::json!({}),
    );
    assert_eq!(disabled["result"]["enabled"], false);
    assert_eq!(disabled["result"]["active"], false);
}

#[test]
fn rpc_retrieval_uses_local_semantic_models_installed_before_workspace_activation() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join(".git")).unwrap();
    std::fs::write(
        workspace.path().join("semantic.rs"),
        "pub fn hidden_semantic_target() -> bool { true }\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("other.rs"),
        "pub fn unrelated_symbol() -> bool { false }\n",
    )
    .unwrap();
    let models = CodebaseModels::new(
        zeta_codebase::EmbeddingIndexKey::new("semantic-test-v1").unwrap(),
        Arc::new(SemanticTestEmbedding),
    );
    let server = server()
        .with_codebase_models(models)
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(
                WorkspaceTrustSource::HostConfiguration,
            ),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path().to_path_buf())
        .unwrap();
    let Ok(codebase) = server.codebase_service() else {
        panic!("Codebase should be installed");
    };
    codebase.rebuild().unwrap();
    server
        .codebase_semantic_service()
        .expect("semantic runtime")
        .sync()
        .expect("semantic sync");
    let mut connection = server.connection();
    call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );

    let retrieval = call(
        &server,
        &mut connection,
        2,
        "workspace/codebase/retrieve",
        serde_json::json!({"query": "conceptual execution flow", "maxResults": 10}),
    );

    assert_eq!(retrieval["result"]["hits"][0]["path"], "semantic.rs");
    assert!(retrieval["result"]["hits"][0].get("origins").is_none());
    assert_eq!(retrieval["result"]["degradations"], serde_json::json!([]));
}

#[test]
fn codebase_model_config_rebinds_after_provider_changes() {
    let workspace = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join(".git")).unwrap();
    std::fs::write(
        workspace.path().join("semantic.rs"),
        "pub fn hidden_semantic_target() -> bool { true }\n",
    )
    .unwrap();
    let config = Arc::new(ConfigStore::open(profile.path().join("config.sqlite3")).unwrap());
    let provider = Arc::new(SemanticTestProvider {
        embedding_runtime_count: AtomicUsize::new(0),
    });
    let provider_trait: Arc<dyn SemanticModelProvider> = provider.clone();
    let server = server()
        .with_config_store(config)
        .with_semantic_model_provider(provider_trait)
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(
                WorkspaceTrustSource::HostConfiguration,
            ),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path().to_path_buf())
        .unwrap();
    let Ok(codebase) = server.codebase_service() else {
        panic!("Codebase should be installed");
    };
    codebase.rebuild().unwrap();
    let mut connection = server.connection();
    call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );

    let configured_provider = call(
        &server,
        &mut connection,
        2,
        "provider/configure",
        serde_json::json!({
            "commandId": "configure-semantic-provider",
            "expectedRevision": 0,
            "config": {
                "provider": "openai-compatible",
                "baseUrl": "https://models-one.example.test/v1",
                "modelContext": {}
            }
        }),
    );
    assert_eq!(configured_provider["result"]["revision"], 1);
    let configured_models = call(
        &server,
        &mut connection,
        3,
        "workspace/codebase/configure",
        serde_json::json!({
            "commandId": "configure-semantic-models",
            "expectedRevision": 1,
            "models": {
                "embeddingModel": {
                    "provider": "openai-compatible",
                    "model": "embed-v1"
                },
                "rerankModel": null
            }
        }),
    );
    assert_eq!(configured_models["result"]["revision"], 2);
    server
        .codebase_semantic_service()
        .expect("configured semantic runtime")
        .sync()
        .unwrap();
    assert_eq!(provider.embedding_runtime_count.load(Ordering::Relaxed), 1);

    let changed_provider = call(
        &server,
        &mut connection,
        4,
        "provider/configure",
        serde_json::json!({
            "commandId": "change-semantic-provider",
            "expectedRevision": 2,
            "config": {
                "provider": "openai-compatible",
                "baseUrl": "https://models-two.example.test/v1",
                "modelContext": {}
            }
        }),
    );
    assert_eq!(changed_provider["result"]["revision"], 3);
    assert!(server.codebase_semantic_service().is_some());
    assert_eq!(provider.embedding_runtime_count.load(Ordering::Relaxed), 2);
    let snapshot = call(
        &server,
        &mut connection,
        5,
        "config/read",
        serde_json::json!({}),
    );
    assert_eq!(
        snapshot["result"]["codebase"]["models"]["embeddingModel"]["model"],
        "embed-v1"
    );
}

#[test]
fn unavailable_rerank_keeps_codebase_model_runtime_inactive() {
    let workspace = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join(".git")).unwrap();
    let config = Arc::new(ConfigStore::open(profile.path().join("config.sqlite3")).unwrap());
    let provider = Arc::new(SemanticTestProvider {
        embedding_runtime_count: AtomicUsize::new(0),
    });
    let provider_trait: Arc<dyn SemanticModelProvider> = provider.clone();
    let server = server()
        .with_config_store(config)
        .with_semantic_model_provider(provider_trait)
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(
                WorkspaceTrustSource::HostConfiguration,
            ),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path().to_path_buf())
        .unwrap();
    let mut connection = server.connection();
    call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );
    call(
        &server,
        &mut connection,
        2,
        "provider/configure",
        serde_json::json!({
            "commandId": "configure-semantic-provider",
            "expectedRevision": 0,
            "config": {
                "provider": "openai-compatible",
                "baseUrl": "https://models.example.test/v1",
                "modelContext": {}
            }
        }),
    );
    let rejected = call(
        &server,
        &mut connection,
        3,
        "workspace/codebase/configure",
        serde_json::json!({
            "commandId": "configure-semantic-models",
            "expectedRevision": 1,
            "models": {
                "embeddingModel": {
                    "provider": "openai-compatible",
                    "model": "embed-v1"
                },
                "rerankModel": {
                    "provider": "openai-compatible",
                    "model": "rerank-v1"
                }
            }
        }),
    );
    assert_eq!(rejected["result"]["revision"], 2);
    let snapshot = call(
        &server,
        &mut connection,
        4,
        "config/read",
        serde_json::json!({}),
    );
    assert_eq!(snapshot["result"]["revision"], 2);
    assert_eq!(
        snapshot["result"]["codebase"]["models"]["rerankModel"]["model"],
        "rerank-v1"
    );
    assert!(server.codebase_semantic_service().is_none());
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
    .unwrap()
}
