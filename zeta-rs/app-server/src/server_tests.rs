use super::*;
use std::sync::Arc;
use zeta_core::{InMemoryJournal, ThreadManager};
use zeta_model_provider::EchoModel;

fn server() -> AppServer {
    AppServer::new(
        Arc::new(ThreadManager::with_journal(Arc::new(
            InMemoryJournal::default(),
        ))),
        Arc::new(EchoModel),
    )
}

fn call(
    server: &AppServer,
    connection: &mut ConnectionState,
    request: serde_json::Value,
) -> serde_json::Value {
    serde_json::from_str(&server.handle_json(connection, &request.to_string())).unwrap()
}

fn initialize(server: &AppServer, connection: &mut ConnectionState) {
    let response = call(
        server,
        connection,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1"},"protocolVersions":{"min":1,"max":1},"capabilities":{}}}),
    );
    assert_eq!(response["result"]["protocolVersion"], 1);
}

#[test]
fn initialize_must_precede_domain_requests() {
    let server = server();
    let response = call(
        &server,
        &mut server.connection(),
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"thread/list","params":{}}),
    );
    assert_eq!(response["error"]["message"], "NotInitialized");
}

#[test]
fn request_ids_must_be_positive_integers() {
    let server = server();
    let response = call(
        &server,
        &mut server.connection(),
        serde_json::json!({"jsonrpc":"2.0","id":"one","method":"initialize","params":{}}),
    );
    assert_eq!(response["error"]["message"], "InvalidRequest");
}

#[test]
fn initialize_validates_client_identity_and_version_range() {
    let server = server();
    let response = call(
        &server,
        &mut server.connection(),
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"","version":"1"},"protocolVersions":{"min":1,"max":1},"capabilities":{}}}),
    );
    assert_eq!(response["error"]["message"], "InvalidParams");
}

#[test]
fn idempotent_thread_start_returns_original_result() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let request = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"thread/start","params":{"idempotencyKey":"one","title":"test"}});
    let first = call(&server, &mut connection, request.clone());
    let second = call(&server, &mut connection, request);
    assert_eq!(first["result"], second["result"]);
    assert_eq!(server.threads().list_threads().unwrap().len(), 1);
}

#[test]
fn side_effecting_requests_reject_empty_idempotency_keys() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let response = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"thread/start","params":{"idempotencyKey":"","title":"test"}}),
    );
    assert_eq!(response["error"]["message"], "InvalidParams");
}

#[test]
fn initialize_rejects_an_unsupported_version_interval() {
    let server = server();
    let response = call(
        &server,
        &mut server.connection(),
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1"},"protocolVersions":{"min":2,"max":3},"capabilities":{}}}),
    );
    assert_eq!(response["error"]["message"], "ProtocolVersionUnsupported");
}

#[test]
fn resources_are_chunked_and_owned_by_one_connection() {
    let server = server();
    let mut owner = server.connection();
    initialize(&server, &mut owner);
    let resource_id = server
        .create_resource(&owner, "text/plain".into(), b"hello".to_vec())
        .unwrap();
    let read = call(
        &server,
        &mut owner,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"resource/read","params":{"resourceId":resource_id,"offset":1,"maxBytes":2}}),
    );
    assert_eq!(read["result"]["data"], serde_json::json!([101, 108]));
    let mut other = server.connection();
    initialize(&server, &mut other);
    let denied = call(
        &server,
        &mut other,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"resource/metadata","params":{"resourceId":resource_id}}),
    );
    assert_eq!(denied["error"]["message"], "ResourceNotOwner");
}

#[test]
fn thread_start_subscribes_connection_and_emits_notification() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let start = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"thread/start","params":{"idempotencyKey":"notify","title":"test"}}),
    );
    let thread_id = start["result"]["threadId"].clone();
    let notifications: Vec<serde_json::Value> = server
        .drain_notifications(&mut connection)
        .into_iter()
        .map(|message| serde_json::from_str(&message).unwrap())
        .collect();
    assert_eq!(notifications[0]["method"], "thread/started");
    call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"turn/start","params":{"idempotencyKey":"turn","threadId":thread_id,"input":[{"type":"text","text":"hello"}]}}),
    );
    let notifications: Vec<serde_json::Value> = server
        .drain_notifications(&mut connection)
        .into_iter()
        .map(|message| serde_json::from_str(&message).unwrap())
        .collect();
    assert_eq!(notifications[0]["method"], "turn/started");
}

#[test]
fn thread_read_does_not_subscribe_but_resume_does() {
    let server = server();
    let mut owner = server.connection();
    initialize(&server, &mut owner);
    let started = call(
        &server,
        &mut owner,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"thread/start","params":{"idempotencyKey":"thread","title":"test"}}),
    );
    let thread_id = started["result"]["threadId"].clone();
    let mut reader = server.connection();
    initialize(&server, &mut reader);
    call(
        &server,
        &mut reader,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"thread/read","params":{"threadId":thread_id}}),
    );
    call(
        &server,
        &mut owner,
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"turn/start","params":{"idempotencyKey":"turn","threadId":started["result"]["threadId"],"input":[{"type":"text","text":"hello"}]}}),
    );
    assert!(server.drain_notifications(&mut reader).is_empty());
    call(
        &server,
        &mut reader,
        serde_json::json!({"jsonrpc":"2.0","id":5,"method":"thread/resume","params":{"threadId":thread_id}}),
    );
    assert!(server.drain_notifications(&mut reader).is_empty());
}
