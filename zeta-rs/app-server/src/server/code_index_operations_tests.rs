use super::*;
use crate::CodeIndexSemanticModels;
use crate::local::ProviderModelService;
use crate::server::WorkspaceSwitchTrustPolicy;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use zeta_config::ConfigStore;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingRuntimeRequest;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider::RerankInvoker;
use zeta_model_provider::RerankRuntimeRequest;
use zeta_model_provider::SemanticModelProvider;
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
    let Ok(code_index) = server.code_index_service() else {
        panic!("code index should be installed");
    };
    code_index.rebuild().unwrap();
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
    assert_eq!(initialize["result"]["capabilities"]["codeIndex"], true);

    let status = call(
        &server,
        &mut connection,
        2,
        "workspace/codeIndex/status",
        serde_json::json!({}),
    );
    assert_eq!(status["result"]["state"], "ready");
    assert!(status["result"]["generation"].as_u64().unwrap() >= 1);
    assert_eq!(status["result"]["semantic"]["state"], "unavailable");

    let search = call(
        &server,
        &mut connection,
        3,
        "workspace/codeIndex/search",
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
        "workspace/codeIndex/retrieve",
        serde_json::json!({"query": "workspace_side_chunking", "maxResults": 10}),
    );
    assert_eq!(retrieval["result"]["hits"][0]["path"], "lib.rs");
    assert_eq!(
        retrieval["result"]["hits"][0]["origins"],
        serde_json::json!(["localLexical"])
    );
    assert_eq!(retrieval["result"]["degradations"], serde_json::json!([]));

    let invalid_limit = call(
        &server,
        &mut connection,
        5,
        "workspace/codeIndex/search",
        serde_json::json!({"query": "workspace_side_chunking", "maxResults": 101}),
    );
    assert_eq!(invalid_limit["error"]["message"], "InvalidParams");

    let invalid_empty_params = call(
        &server,
        &mut connection,
        6,
        "workspace/codeIndex/status",
        serde_json::json!({"unexpected": true}),
    );
    assert_eq!(invalid_empty_params["error"]["message"], "InvalidParams");
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
    let models = CodeIndexSemanticModels::new(
        zeta_code_index_semantic::CodeIndexEmbeddingModelId::new("semantic-test-v1").unwrap(),
        Arc::new(SemanticTestEmbedding),
    );
    let server = server()
        .with_code_index_semantic_models(models)
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
    let Ok(code_index) = server.code_index_service() else {
        panic!("code index should be installed");
    };
    code_index.rebuild().unwrap();
    server
        .code_index_semantic_service()
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

    let retry = call(
        &server,
        &mut connection,
        9,
        "workspace/codeIndex/semantic/retry",
        serde_json::json!({}),
    );
    assert!(matches!(
        retry["result"]["semantic"]["state"].as_str(),
        Some("stale" | "syncing" | "ready")
    ));
    let cancelled = call(
        &server,
        &mut connection,
        10,
        "workspace/codeIndex/semantic/cancel",
        serde_json::json!({}),
    );
    assert!(matches!(
        cancelled["result"]["semantic"]["state"].as_str(),
        Some("cancelled" | "ready")
    ));

    let retrieval = call(
        &server,
        &mut connection,
        2,
        "workspace/codeIndex/retrieve",
        serde_json::json!({"query": "conceptual execution flow", "maxResults": 10}),
    );

    assert_eq!(retrieval["result"]["hits"][0]["path"], "semantic.rs");
    assert_eq!(
        retrieval["result"]["hits"][0]["origins"],
        serde_json::json!(["localSemantic"])
    );
    assert_eq!(retrieval["result"]["degradations"], serde_json::json!([]));
}

#[test]
fn semantic_config_requires_workspace_consent_and_rebinds_after_provider_changes() {
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
    let Ok(code_index) = server.code_index_service() else {
        panic!("code index should be installed");
    };
    code_index.rebuild().unwrap();
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
        "workspace/codeIndex/semantic/configure",
        serde_json::json!({
            "commandId": "configure-semantic-models",
            "expectedRevision": 1,
            "selection": {
                "type": "remote",
                "models": {
                    "embeddingModel": {
                        "provider": "openai-compatible",
                        "model": "embed-v1"
                    },
                    "rerankModel": null
                }
            }
        }),
    );
    assert_eq!(configured_models["result"]["revision"], 2);
    assert!(server.code_index_semantic_service().is_none());
    assert_eq!(provider.embedding_runtime_count.load(Ordering::Relaxed), 0);

    let unauthorized = call(
        &server,
        &mut connection,
        4,
        "config/read",
        serde_json::json!({}),
    );
    assert_eq!(
        unauthorized["result"]["semanticCodeIndex"]["activeWorkspaceAuthorized"],
        false
    );

    let authorized = call(
        &server,
        &mut connection,
        5,
        "workspace/codeIndex/semantic/authorize",
        serde_json::json!({
            "commandId": "authorize-semantic-models",
            "expectedRevision": 2
        }),
    );
    assert_eq!(authorized["result"]["revision"], 3);
    server
        .code_index_semantic_service()
        .expect("authorized semantic runtime")
        .sync()
        .unwrap();
    assert_eq!(provider.embedding_runtime_count.load(Ordering::Relaxed), 2);

    let changed_provider = call(
        &server,
        &mut connection,
        6,
        "provider/configure",
        serde_json::json!({
            "commandId": "change-semantic-provider",
            "expectedRevision": 3,
            "config": {
                "provider": "openai-compatible",
                "baseUrl": "https://models-two.example.test/v1",
                "modelContext": {}
            }
        }),
    );
    assert_eq!(changed_provider["result"]["revision"], 4);
    assert!(server.code_index_semantic_service().is_none());
    let invalidated = call(
        &server,
        &mut connection,
        7,
        "config/read",
        serde_json::json!({}),
    );
    assert_eq!(
        invalidated["result"]["semanticCodeIndex"]["activeWorkspaceAuthorized"],
        false
    );
}

#[test]
fn semantic_authorization_rejects_an_unavailable_rerank_before_recording_consent() {
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
    call(
        &server,
        &mut connection,
        3,
        "workspace/codeIndex/semantic/configure",
        serde_json::json!({
            "commandId": "configure-semantic-models",
            "expectedRevision": 1,
            "selection": {
                "type": "remote",
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
            }
        }),
    );

    let rejected = call(
        &server,
        &mut connection,
        4,
        "workspace/codeIndex/semantic/authorize",
        serde_json::json!({
            "commandId": "authorize-unavailable-rerank",
            "expectedRevision": 2
        }),
    );

    assert_eq!(rejected["error"]["message"], "CodeIndexOperationFailed");
    let snapshot = call(
        &server,
        &mut connection,
        5,
        "config/read",
        serde_json::json!({}),
    );
    assert_eq!(snapshot["result"]["revision"], 2);
    assert_eq!(
        snapshot["result"]["semanticCodeIndex"]["activeWorkspaceAuthorized"],
        false
    );
    assert!(server.code_index_semantic_service().is_none());
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
