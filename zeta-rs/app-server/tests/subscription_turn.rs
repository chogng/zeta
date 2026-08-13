#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use zeta_app_server::AppServer;
use zeta_app_server::ConnectionState;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::SessionStateMode;
use zeta_app_server::open_local_app_server;
use zeta_codex_app_server::CodexAppServerOptions;

#[test]
fn product_routes_selected_subscription_model_through_codex_turns() {
    let profile = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let canonical_workspace = workspace.path().canonicalize().unwrap();
    let (codex_root, codex_program) = fake_subscription_codex_program();
    let server = open_local_app_server(
        LocalAppServerOptions::new(profile.path())
            .with_workspace_root(workspace.path())
            .without_built_in_skills()
            .with_session_state_mode(SessionStateMode::Ephemeral)
            .with_codex_app_server(
                CodexAppServerOptions::new(codex_program)
                    .with_request_timeout(Duration::from_secs(10)),
            ),
    )
    .unwrap();
    let mut connection = server.connection();

    let initialized = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}
        }),
    );
    assert!(initialized.get("result").is_some());
    let listed = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"model/list","params":{}}),
    );
    assert!(
        listed["result"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| {
                entry["model"]["provider"] == "openai-chatgpt"
                    && entry["model"]["model"] == "gpt-codex-product-test"
            })
    );

    let created = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"session/create",
            "params":{"commandId":"create-codex-session","title":"Codex product route"}
        }),
    );
    let session_id = created["result"]["session"]["sessionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let selected = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"select-codex-model",
                "sessionId":session_id,
                "expectedSequence":created["result"]["session"]["sequence"],
                "request":{"type":"setModel","model":{"provider":"openai-chatgpt","model":"gpt-codex-product-test"}}
            }
        }),
    );
    assert_eq!(
        selected["result"]["value"]["session"]["model"]["provider"],
        "openai-chatgpt"
    );
    let thread = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"session/request",
            "params":{
                "commandId":"create-codex-thread",
                "sessionId":session_id,
                "expectedSequence":selected["result"]["value"]["session"]["sequence"],
                "request":{"type":"createThread","title":"root"}
            }
        }),
    );
    let thread_id = thread["result"]["value"]["threadId"]
        .as_str()
        .unwrap()
        .to_owned();
    let started = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"session/request",
            "params":{
                "commandId":"start-codex-product-turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{
                    "type":"startTurn",
                    "threadId":thread_id,
                    "approvalMode":"askPermissions",
                    "input":[{"type":"text","text":"inspect the product workspace"}]
                }
            }
        }),
    );
    assert_eq!(
        started["result"]["type"], "turn",
        "start response: {started}"
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut read_request_id = 7_u64;
    loop {
        let read = call(
            &server,
            &mut connection,
            serde_json::json!({
                "jsonrpc":"2.0","id":read_request_id,"method":"session/thread/read",
                "params":{"sessionId":session_id,"threadId":thread_id}
            }),
        );
        read_request_id += 1;
        let latest = read["result"]["thread"]["turns"]
            .as_array()
            .and_then(|turns| turns.last());
        if read.get("error").is_some() {
            panic!("thread read failed: {read}");
        }
        if latest.is_some_and(|turn| turn["status"] == "completed") {
            assert!(
                latest.unwrap()["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| {
                        item["type"] == "agentMessage" && item["text"] == "Product route complete"
                    })
            );
            break;
        }
        if latest.is_some_and(|turn| turn["status"] == "failed") {
            let requests =
                std::fs::read_to_string(codex_root.path().join("requests.log")).unwrap_or_default();
            panic!("Codex product Turn failed: {latest:?}; requests: {requests}");
        }
        assert!(
            Instant::now() < deadline,
            "Codex product Turn did not complete: {latest:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }

    let requests = std::fs::read_to_string(codex_root.path().join("requests.log")).unwrap();
    let requests = requests
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect::<Vec<_>>();
    let thread_start = requests
        .iter()
        .find(|request| request["method"] == "thread/start")
        .unwrap();
    assert_eq!(thread_start["params"]["model"], "gpt-codex-product-test");
    assert_eq!(
        thread_start["params"]["cwd"],
        canonical_workspace.to_str().unwrap()
    );
    assert_eq!(thread_start["params"]["sandbox"], "workspace-write");
    assert_eq!(thread_start["params"]["approvalPolicy"], "on-request");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request["method"] == "turn/start")
            .count(),
        1
    );
}

fn call(
    server: &AppServer,
    connection: &mut ConnectionState,
    request: serde_json::Value,
) -> serde_json::Value {
    serde_json::from_str(&server.handle_json(connection, &request.to_string())).unwrap()
}

fn fake_subscription_codex_program() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("codex");
    let request_log = root.path().join("requests.log");
    let script = format!(
        r#"#!/bin/sh
while IFS= read -r line; do
  printf '%s\n' "$line" >> '{request_log}'
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"userAgent\":\"codex-product-test/1.0\",\"codexHome\":\"/tmp/codex-product-test\",\"platformFamily\":\"unix\",\"platformOs\":\"test\"}}}}"
      ;;
    *'"method":"account/read"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"account\":{{\"type\":\"chatgpt\",\"email\":\"hidden@example.invalid\"}}}}}}"
      ;;
    *'"method":"model/list"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"data\":[{{\"model\":\"gpt-codex-product-test\",\"displayName\":\"Codex Product Test\",\"isDefault\":true}}],\"nextCursor\":null}}}}"
      ;;
    *'"method":"thread/start"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"thread\":{{\"id\":\"product-remote-thread\"}}}}}}"
      ;;
    *'"method":"turn/start"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"turn\":{{\"id\":\"product-remote-turn\",\"status\":\"inProgress\",\"items\":[]}}}}}}"
      printf '%s\n' '{{"jsonrpc":"2.0","method":"turn/started","params":{{"threadId":"product-remote-thread","turn":{{"id":"product-remote-turn","status":"inProgress","items":[]}}}}}}'
      printf '%s\n' '{{"jsonrpc":"2.0","method":"item/agentMessage/delta","params":{{"threadId":"product-remote-thread","turnId":"product-remote-turn","itemId":"product-message","delta":"Product route complete"}}}}'
      printf '%s\n' '{{"jsonrpc":"2.0","method":"turn/completed","params":{{"threadId":"product-remote-thread","turn":{{"id":"product-remote-turn","status":"completed","items":[],"error":null}}}}}}'
      ;;
  esac
done
"#,
        request_log = request_log.display(),
    );
    std::fs::write(&program, script).unwrap();
    let mut permissions = std::fs::metadata(&program).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&program, permissions).unwrap();
    (root, program)
}
