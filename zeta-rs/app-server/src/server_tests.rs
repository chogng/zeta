use super::*;
use base64::Engine;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zeta_app_server_protocol::protocol::slash_commands::{
    SlashCommandArgumentModeDto, SlashCommandDefinition,
};
use zeta_async_utils::CancellationToken;
use zeta_config::ConfigStore;
use zeta_core::{
    CoreError, InMemorySessionStore, InMemoryThreadStore, ModelService, RequestTurnInteraction,
    SessionCoordinator, StartTurnRequest, ThreadController,
};
use zeta_file_system::LocalFileSystem;
use zeta_model_provider::EchoModel;
use zeta_protocol::{
    ActionApprovalCapability, ActionApprovalCapabilityKind, ActionApprovalRequest, AgentRequest,
    CommandId, ContentPart, InputItem, ModelRequest, ModelResponse, RequestId, RequestUserInput,
    ResponseItem, StopReason, TurnStatus, UserInput,
};
use zeta_sandboxing::WorkspaceRoot;

fn server_with_model(model: Arc<dyn ModelService>) -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    AppServer::new(sessions, model)
}

fn server() -> AppServer {
    server_with_model(Arc::new(crate::local::ProviderModelService::new(Arc::new(
        EchoModel,
    ))))
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
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}
        }),
    );
    assert_eq!(response["result"]["capabilities"]["sessions"], true);
    assert_eq!(response["result"]["capabilities"]["typst"], true);
    assert_eq!(response["result"]["capabilities"]["updateReplay"], true);
}

fn create_session(
    server: &AppServer,
    connection: &mut ConnectionState,
    request_id: u64,
    command_id: &str,
) -> serde_json::Value {
    call(
        server,
        connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":request_id,
            "method":"session/create",
            "params":{"commandId":command_id,"title":"task"}
        }),
    )
}

fn create_thread(
    server: &AppServer,
    connection: &mut ConnectionState,
    request_id: u64,
    command_id: &str,
    session_id: &str,
    expected_sequence: u64,
) -> serde_json::Value {
    call(
        server,
        connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":request_id,
            "method":"session/thread/create",
            "params":{
                "commandId":command_id,
                "sessionId":session_id,
                "expectedSequence":expected_sequence,
                "title":"root"
            }
        }),
    )
}

fn wait_for_latest_turn(server: &AppServer, thread_id: &str, expected: TurnStatus) {
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let snapshot = server.sessions().threads().read_thread(&thread_id).unwrap();
        if snapshot
            .turns
            .last()
            .is_some_and(|turn| turn.status == expected)
        {
            return;
        }
        assert!(Instant::now() < deadline, "Turn did not reach {expected:?}");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn initialize_is_required_and_request_ids_are_connection_unique() {
    let server = server();
    let mut connection = server.connection();
    let gated = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"session/list","params":{}}),
    );
    assert_eq!(gated["error"]["message"], "NotInitialized");

    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let duplicate = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"session/list","params":{}}),
    );
    assert_eq!(duplicate["error"]["message"], "InvalidRequest");
}

#[test]
fn workspace_search_requires_an_installed_backend() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let response = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"workspace/search/start",
            "params":{
                "query":"needle",
                "patternKind":"literal",
                "caseSensitivity":"smart",
                "includePatterns":[],
                "excludePatterns":[],
                "maxResults":100
            }
        }),
    );

    assert_eq!(response["error"]["message"], "SearchUnavailable");
}

#[test]
fn initialize_advertises_the_server_slash_command_snapshot() {
    let catalog = SlashCommandCatalog::new([SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentModeDto::Optional,
    }])
    .unwrap();
    let server = server().with_slash_command_catalog(catalog);
    let mut connection = server.connection();

    let response = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}
        }),
    );

    assert_eq!(
        response["result"]["slashCommands"],
        serde_json::json!([{
            "name": "diagnose",
            "description": "inspect the current workspace",
            "argumentMode": "optional"
        }])
    );
}

#[test]
fn session_first_flow_exposes_canonical_session_and_thread_models() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "create-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    assert_eq!(session["result"]["session"]["sequence"], 1);
    let thread = create_thread(&server, &mut connection, 3, "create-thread", session_id, 1);
    let thread_id = thread["result"]["threadId"].as_str().unwrap();

    assert_eq!(
        thread["result"]["session"]["threads"][0]["status"],
        "active"
    );
    let read = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"thread/read",
            "params":{"threadId":thread_id}
        }),
    );
    assert_eq!(read["result"]["thread"]["sessionId"], session_id);
    assert_eq!(read["result"]["thread"]["sequence"], 1);
}

#[test]
fn typed_commands_replay_and_reject_payload_conflicts() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let first = create_session(&server, &mut connection, 2, "same-command");
    let replayed = create_session(&server, &mut connection, 3, "same-command");
    assert_eq!(
        replayed["result"]["session"]["sessionId"],
        first["result"]["session"]["sessionId"]
    );
    let conflict = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/create",
            "params":{"commandId":"same-command","title":"different"}
        }),
    );
    assert_eq!(conflict["error"]["message"], "CommandConflict");
}

#[test]
fn fork_freezes_parent_thread_sequence_in_session_lineage() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let root = create_thread(&server, &mut connection, 3, "root", session_id, 1);
    let root_id = root["result"]["threadId"].as_str().unwrap();
    let fork = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/fork",
            "params":{
                "commandId":"fork",
                "sessionId":session_id,
                "expectedSequence":3,
                "parentThreadId":root_id,
                "title":"branch"
            }
        }),
    );

    assert_eq!(
        fork["result"]["session"]["threads"][1]["origin"]["type"],
        "fork"
    );
    assert_eq!(
        fork["result"]["session"]["threads"][1]["origin"]["parentSequence"],
        1
    );
}

#[derive(Default)]
struct CountingModel {
    calls: AtomicUsize,
}

impl ModelService for CountingModel {
    fn invoke(
        &self,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let prompt = request
            .input
            .iter()
            .find_map(|item| match item {
                InputItem::Message(message) => message.content.iter().find_map(|content| {
                    let ContentPart::Text(text) = content else {
                        return None;
                    };
                    Some(text.as_str())
                }),
                InputItem::ToolResult(_) => None,
            })
            .unwrap_or_default();
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(format!("answer: {prompt}"))],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

#[test]
fn completed_turn_replays_without_invoking_the_model_twice() {
    let model = Arc::new(CountingModel::default());
    let server = server_with_model(model.clone());
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["threadId"].as_str().unwrap();
    let request = |id| {
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":id,
            "method":"turn/start",
            "params":{
                "commandId":"turn",
                "sessionId":session_id,
                "threadId":thread_id,
                "expectedSequence":1,
                "input":[{"type":"text","text":"hello"}]
            }
        })
    };
    let first = call(&server, &mut connection, request(4));
    let replayed = call(&server, &mut connection, request(5));

    assert_eq!(first["result"]["turnId"], replayed["result"]["turnId"]);
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    assert_eq!(model.calls.load(Ordering::Relaxed), 1);
    let notifications = server.drain_notifications(&mut connection);
    assert!(notifications.iter().any(|notification| {
        notification.contains("\"method\":\"thread/update\"")
            && notification.contains("\"agentMessage\"")
    }));
}

#[test]
fn updates_are_broadcast_to_other_subscribed_connections() {
    let server = server();
    let mut writer = server.connection();
    initialize(&server, &mut writer);
    let session = create_session(&server, &mut writer, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut writer, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["threadId"].as_str().unwrap();

    let mut observer = server.connection();
    initialize(&server, &mut observer);
    call(
        &server,
        &mut observer,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"session/subscribe",
            "params":{"sessionId":session_id,"afterSequence":3}
        }),
    );
    call(
        &server,
        &mut observer,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"thread/subscribe",
            "params":{"threadId":thread_id,"afterSequence":1}
        }),
    );
    call(
        &server,
        &mut writer,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/thread/fork",
            "params":{
                "commandId":"fork",
                "sessionId":session_id,
                "expectedSequence":3,
                "parentThreadId":thread_id,
                "title":"branch"
            }
        }),
    );
    call(
        &server,
        &mut writer,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"turn/start",
            "params":{
                "commandId":"turn",
                "sessionId":session_id,
                "threadId":thread_id,
                "expectedSequence":1,
                "input":[{"type":"text","text":"hello"}]
            }
        }),
    );
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);

    let notifications = server.drain_notifications(&mut observer);
    assert!(
        notifications
            .iter()
            .any(|value| value.contains("\"method\":\"session/update\""))
    );
    assert!(notifications.iter().any(|value| {
        value.contains("\"method\":\"thread/update\"") && value.contains("\"agentMessage\"")
    }));
    assert!(notifications.iter().any(|value| {
        value.contains("\"method\":\"thread/update\"")
            && value.contains("\"itemDelta\"")
            && value.contains("\"streamCursor\"")
    }));
}

#[test]
fn subscribe_returns_durable_gap_for_reconnect() {
    let server = server();
    let mut first_connection = server.connection();
    initialize(&server, &mut first_connection);
    let session = create_session(&server, &mut first_connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut first_connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["threadId"].as_str().unwrap();

    let mut reconnected = server.connection();
    initialize(&server, &mut reconnected);
    let replay = call(
        &server,
        &mut reconnected,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"thread/subscribe",
            "params":{"threadId":thread_id,"afterSequence":0}
        }),
    );
    assert_eq!(replay["result"]["updates"][0]["durableSequence"], 1);
    assert_eq!(
        replay["result"]["updates"][0]["update"]["type"],
        "committed"
    );
}

#[test]
fn resources_remain_connection_owned_and_chunked() {
    let server = server();
    let mut owner = server.connection();
    let mut other = server.connection();
    initialize(&server, &mut owner);
    initialize(&server, &mut other);
    let resource_id = server
        .create_resource(&owner, "text/plain".into(), b"hello".to_vec())
        .unwrap();
    let owner_read = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"resource/read",
            "params":{"resourceId":resource_id,"offset":0,"maxBytes":3}
        }),
    );
    assert_eq!(owner_read["result"]["decodedLength"], 3);
    let denied = call(
        &server,
        &mut other,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"resource/read",
            "params":{"resourceId":resource_id,"offset":0,"maxBytes":3}
        }),
    );
    assert_eq!(denied["error"]["message"], "ResourceNotOwner");
}

#[test]
fn typst_compilation_returns_a_connection_owned_pdf_resource() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let compiled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"document/typst/compile",
            "params":{"source":"= Paper\n\nA formula: $x^2$."}
        }),
    );
    assert_eq!(compiled["result"]["status"], "success");
    assert_eq!(
        compiled["result"]["resource"]["mimeType"],
        "application/pdf"
    );
    let resource_id = compiled["result"]["resource"]["resourceId"]
        .as_str()
        .unwrap();

    let bytes = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"resource/read",
            "params":{"resourceId":resource_id,"offset":0,"maxBytes":16}
        }),
    );
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(bytes["result"]["dataBase64"].as_str().unwrap())
        .unwrap();
    assert!(decoded.starts_with(b"%PDF-"));
}

#[test]
fn typst_source_errors_are_typed_results_not_server_failures() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let compiled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"document/typst/compile",
            "params":{"source":"#let ="}
        }),
    );
    assert_eq!(compiled["result"]["status"], "failed");
    assert!(
        compiled["result"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty())
    );
}

#[test]
fn config_updates_use_typed_command_ids() {
    let path = std::env::temp_dir().join(format!(
        "zeta-app-server-config-{}.json",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let server = server().with_config_store(Arc::new(ConfigStore::open(&path).unwrap()));
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let updated = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"config/update",
            "params":{
                "commandId":"theme","expectedRevision":0,"theme":"dark",
                "approvalReviewModel":{"type":"automatic"}
            }
        }),
    );
    assert_eq!(updated["result"]["revision"], 1);
    assert_eq!(updated["result"]["generation"], 1);
    assert_eq!(updated["result"]["disposition"], "updated");
    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"config/read","params":{}}),
    );
    assert_eq!(read["result"]["revision"], 1);
    assert_eq!(read["result"]["theme"], "dark");
    assert_eq!(
        read["result"]["approvalReviewModel"],
        serde_json::json!({"type":"automatic"})
    );
    let mcp = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"mcp/server/upsert",
            "params":{
                "commandId":"github-mcp","expectedRevision":1,
                "server":{
                    "id":"user:mcp:github",
                    "displayName":"GitHub",
                    "transport":{"type":"streamableHttp","url":"https://mcp.github.example"},
                    "credential":{"type":"reference","credentialRef":"user:credential:github"},
                    "enablement":"disabled"
                }
            }
        }),
    );
    assert_eq!(mcp["result"]["revision"], 2);
    let skill = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"skill/source/add",
            "params":{
                "commandId":"personal-skills","expectedRevision":2,
                "source":{
                    "id":"user:skill-source:personal",
                    "rootReference":"user:skill-root:personal",
                    "enablement":"disabled"
                }
            }
        }),
    );
    assert_eq!(skill["result"]["revision"], 3);
    let enabled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"mcp/server/enablement/set",
            "params":{
                "commandId":"enable-github-mcp","expectedRevision":3,
                "serverId":"user:mcp:github","enablement":"enabled"
            }
        }),
    );
    assert_eq!(enabled["result"]["revision"], 4);
    let stale = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":7,"method":"skill/source/enablement/set",
            "params":{
                "commandId":"stale-skill","expectedRevision":3,
                "sourceId":"user:skill-source:personal","enablement":"enabled"
            }
        }),
    );
    assert_eq!(stale["error"]["message"], "ConfigRevisionConflict");
    let configured = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":8,"method":"config/read","params":{}}),
    );
    assert_eq!(configured["result"]["revision"], 4);
    assert_eq!(
        configured["result"]["mcpServers"]["user:mcp:github"]["enablement"],
        "enabled"
    );
    assert_eq!(
        configured["result"]["skillSources"]["user:skill-source:personal"]["rootReference"],
        "user:skill-root:personal"
    );
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("lock"));
    let _ = std::fs::remove_file(path.with_extension("tmp"));
}

#[test]
fn interaction_resolution_uses_the_durable_request_identity() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["threadId"].as_str().unwrap();
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let session_id = zeta_protocol::SessionId::new(session_id).unwrap();
    let started = server
        .sessions()
        .threads()
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("agent-turn").unwrap(),
                expected_sequence: zeta_core::SequenceExpectation::Exact(1),
                input: vec![UserInput::Text {
                    text: "wait".into(),
                }],
            },
        )
        .unwrap();
    server
        .sessions()
        .threads()
        .request_turn_interaction(
            &thread_id,
            &started.turn_id,
            RequestTurnInteraction {
                request_id: RequestId::new("input-1").unwrap(),
                item_id: None,
                request: AgentRequest::UserInput {
                    request: RequestUserInput {
                        questions: Vec::new(),
                    },
                },
                deadline: None,
            },
        )
        .unwrap();

    let resolved = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"turn/interaction/resolve",
            "params":{
                "commandId":"resolve-input-1",
                "sessionId":session_id,
                "threadId":thread_id,
                "turnId":started.turn_id,
                "requestId":"input-1",
                "expectedSequence":5,
                "response":{"type":"userInput", "response":{"answers":{}}}
            }
        }),
    );

    assert_eq!(resolved["result"]["sequence"], 6);
    let snapshot = server.sessions().threads().read_thread(&thread_id).unwrap();
    assert_eq!(snapshot.turns[0].status, zeta_core::TurnStatus::Running);
    assert!(snapshot.turns[0].pending_interaction.is_none());
}

#[test]
fn approval_interaction_resolves_through_the_typed_app_server_contract() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["threadId"].as_str().unwrap();
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let session_id = zeta_protocol::SessionId::new(session_id).unwrap();
    let started = server
        .sessions()
        .threads()
        .start_turn(
            &thread_id,
            StartTurnRequest {
                command_id: CommandId::new("approval-turn").unwrap(),
                expected_sequence: zeta_core::SequenceExpectation::Exact(1),
                input: vec![UserInput::Text {
                    text: "approve".into(),
                }],
            },
        )
        .unwrap();
    server
        .sessions()
        .threads()
        .request_turn_interaction(
            &thread_id,
            &started.turn_id,
            RequestTurnInteraction {
                request_id: RequestId::new("approval-1").unwrap(),
                item_id: None,
                request: AgentRequest::Approval {
                    request: ActionApprovalRequest {
                        action_digest: "a".repeat(64),
                        policy_revision: "policy-1".into(),
                        capabilities: vec![ActionApprovalCapability {
                            kind: ActionApprovalCapabilityKind::Network,
                            scope: "api.example.com".into(),
                        }],
                        reason: "network requires approval".into(),
                    },
                },
                deadline: None,
            },
        )
        .unwrap();

    let resolved = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"turn/interaction/resolve",
            "params":{
                "commandId":"resolve-approval-1",
                "sessionId":session_id,
                "threadId":thread_id,
                "turnId":started.turn_id,
                "requestId":"approval-1",
                "expectedSequence":5,
                "response":{
                    "type":"approval",
                    "response":{"decision":"approveOnce"}
                }
            }
        }),
    );

    assert_eq!(resolved["result"]["sequence"], 6);
    let events = server
        .sessions()
        .threads()
        .thread_updates_after(&thread_id, 5)
        .unwrap();
    assert!(matches!(
        &events[0].update,
        zeta_protocol::ThreadUpdate::Committed {
            event: zeta_protocol::ThreadEvent::InteractionResolved {
                response: zeta_protocol::AgentResponse::Approval { .. },
                ..
            }
        }
    ));
}

#[test]
fn jsonl_transport_writes_response_before_causal_updates() {
    let server = server();
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"},\"capabilities\":{}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"session/create\",\"params\":{\"commandId\":\"session\",\"title\":\"task\"}}\n"
    );
    let mut output = Vec::new();
    server
        .serve_jsonl(Cursor::new(input.as_bytes()), &mut output)
        .unwrap();
    let lines = String::from_utf8(output).unwrap();
    assert_eq!(lines.lines().count(), 2);
    assert!(lines.lines().all(|line| line.contains("\"id\":")));
}

#[test]
fn filesystem_rpc_lists_and_describes_workspace_paths() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-files-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "hello").unwrap();
    let server = server().with_file_system(Arc::new(LocalFileSystem::new(
        WorkspaceRoot::open(&root).unwrap(),
    )));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let listed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"fs/readDirectory",
            "params":{"path":"src"}
        }),
    );
    let metadata = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"fs/getMetadata",
            "params":{"path":"src/lib.rs"}
        }),
    );
    let contents = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"fs/readFile",
            "params":{"path":"src/lib.rs"}
        }),
    );

    assert_eq!(
        listed["result"]["entries"],
        serde_json::json!([{"name":"lib.rs","fileType":"file"}]),
    );
    assert_eq!(metadata["result"]["fileType"], "file");
    assert_eq!(metadata["result"]["sizeBytes"], 5);
    assert_eq!(contents["result"]["content"], "hello");
    let _ = std::fs::remove_dir_all(root);
}
