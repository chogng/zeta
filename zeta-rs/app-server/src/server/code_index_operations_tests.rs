use super::*;
use crate::local::ProviderModelService;
use crate::server::WorkspaceSwitchTrustPolicy;
use std::sync::Arc;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;
use zeta_workspace::WorkspaceTrustSource;

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
