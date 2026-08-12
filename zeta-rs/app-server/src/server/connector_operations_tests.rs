use super::*;
use crate::local::ProviderModelService;
use std::sync::Arc;
use std::time::Duration;
use zeta_connectors::ConnectorDefinition;
use zeta_connectors::ConnectorId;
use zeta_connectors::ConnectorRuntimeBinding;
use zeta_connectors_extension::ConnectorAuthority;
use zeta_connectors_extension::ConnectorCredentialService;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;
use zeta_secrets::MemorySecretStore;

fn server() -> AppServer {
    let connector = ConnectorDefinition::new(
        ConnectorId::new("acme/github:connector:account").unwrap(),
        "GitHub",
        "GitHub tools",
        ConnectorRuntimeBinding::mcp_server("plugin:acme/github:mcp:github").unwrap(),
    )
    .unwrap();
    let authority = ConnectorAuthority::in_memory([connector]).unwrap();
    let service = Arc::new(ConnectorCredentialService::new(
        authority,
        Arc::new(MemorySecretStore::default()),
    ));
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
    .with_connector_service(service)
}

fn call(
    server: &AppServer,
    connection: &mut crate::server::ConnectionState,
    id: u64,
    method: &str,
    params: Value,
) -> Value {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    serde_json::from_str(&server.handle_json(connection, &request.to_string())).unwrap()
}

#[test]
fn app_server_connects_lists_notifies_and_disconnects_without_projecting_secrets() {
    let server = server();
    let mut connection = server.connection();
    let initialized = call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );
    assert!(initialized.get("result").is_some());
    assert_eq!(initialized["result"]["capabilities"]["connectors"], true);

    let listed = call(
        &server,
        &mut connection,
        2,
        "connector/list",
        serde_json::json!({}),
    );
    assert_eq!(listed["result"]["generation"], 1);
    assert_eq!(
        listed["result"]["connectors"][0]["state"]["status"],
        "disconnected"
    );

    let connected = call(
        &server,
        &mut connection,
        3,
        "connector/connect/apiToken",
        serde_json::json!({
            "commandId": "connect-github",
            "expectedGeneration": 1,
            "connectorId": "acme/github:connector:account",
            "connectionGeneration": 1,
            "accountId": "octocat",
            "accountDisplayName": "Octocat",
            "apiToken": "super-secret-token"
        }),
    );
    assert_eq!(connected["result"]["generation"], 3);
    assert_eq!(connected["result"]["disposition"], "updated");

    let connected_list = call(
        &server,
        &mut connection,
        4,
        "connector/list",
        serde_json::json!({}),
    );
    let serialized = connected_list.to_string();
    assert!(!serialized.contains("super-secret-token"));
    assert!(!serialized.contains("credential"));
    assert_eq!(
        connected_list["result"]["connectors"][0]["state"]["status"],
        "connected"
    );

    let notifications = server.connection_notifications(&connection);
    let mut changed = Vec::new();
    for _ in 0..100 {
        changed.extend(notifications.drain());
        if changed.iter().any(|notification| {
            serde_json::from_str::<Value>(notification)
                .is_ok_and(|value| value["method"] == "connector/changed")
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(changed.iter().any(|notification| {
        serde_json::from_str::<Value>(notification)
            .is_ok_and(|value| value["method"] == "connector/changed")
    }));

    let disconnected = call(
        &server,
        &mut connection,
        5,
        "connector/disconnect",
        serde_json::json!({
            "commandId": "disconnect-github",
            "expectedGeneration": 3,
            "connectorId": "acme/github:connector:account"
        }),
    );
    assert_eq!(disconnected["result"]["command"]["generation"], 4);
    assert_eq!(disconnected["result"]["credentialCleanup"], "deleted");
}
