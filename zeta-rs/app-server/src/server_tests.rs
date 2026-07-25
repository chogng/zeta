use super::*;
use std::io::Cursor;
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
fn request_ids_must_be_unique_within_a_connection() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let response = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"thread/list","params":{}}),
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
    let mut retry = request;
    retry["id"] = serde_json::json!(3);
    let second = call(&server, &mut connection, retry);
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
fn config_methods_round_trip_a_provider_scoped_model_reference() {
    let path = std::env::temp_dir().join(format!(
        "zeta-app-server-config-model-ref-{}.json",
        std::process::id()
    ));
    let server =
        server().with_config_store(Arc::new(zeta_config::ConfigStore::open(&path).unwrap()));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let updated = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "config/update",
            "params": {
                "idempotencyKey": "model-ref",
                "preferredModel": {
                    "provider": "openai",
                    "model": "gpt-5.6"
                }
            }
        }),
    );
    assert_eq!(
        updated["result"]["preferredModel"],
        serde_json::json!({"provider": "openai", "model": "gpt-5.6"})
    );

    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"config/read","params":{}}),
    );
    assert_eq!(
        read["result"]["preferredModel"],
        updated["result"]["preferredModel"]
    );

    let invalid = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "config/update",
            "params": {
                "idempotencyKey": "invalid-model-ref",
                "preferredModel": {
                    "provider": "",
                    "model": "gpt-5.6"
                }
            }
        }),
    );
    assert_eq!(invalid["error"]["message"], "InvalidParams");
    let _ = std::fs::remove_file(path);
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

#[test]
fn jsonl_connection_writes_responses_before_their_notifications() {
    let server = server();
    let thread_id = server.threads().start_thread("test").unwrap();
    let requests = [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1"},"protocolVersions":{"min":1,"max":1},"capabilities":{}}}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"thread/resume","params":{"threadId":thread_id}}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"turn/start","params":{"idempotencyKey":"turn","threadId":thread_id,"input":[{"type":"text","text":"hello"}]}}),
    ]
    .into_iter()
    .map(|request| request.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    let mut output = Vec::new();
    server
        .serve_jsonl(
            Cursor::new(format!("{requests}\n").into_bytes()),
            &mut output,
        )
        .unwrap();
    let messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(messages[0]["id"], 1);
    assert_eq!(messages[1]["id"], 2);
    assert_eq!(messages[2]["id"], 3);
    assert_eq!(messages[3]["method"], "turn/started");
    assert_eq!(messages[4]["method"], "item/agentMessage/completed");
    assert_eq!(messages[5]["method"], "turn/completed");
}
