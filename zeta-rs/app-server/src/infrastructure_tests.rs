use super::call;
use super::create_session;
use super::create_thread;
use super::initialize;
use super::server;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

fn queued_server(path: &std::path::Path) -> (crate::AppServer, Arc<queue::QueueStore>) {
    let store = Arc::new(queue::QueueStore::open(&path.join("state.sqlite")).unwrap());
    (
        server()
            .with_queue_store(store.clone(), Some(path.to_str().unwrap().into()))
            .unwrap(),
        store,
    )
}

fn create(server: &crate::AppServer, connection: &mut crate::ConnectionState) -> (String, String) {
    let initialized = call(
        server,
        connection,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"queue-test","version":"1"},"capabilities":{}}}),
    );
    assert_eq!(
        initialized["result"]["capabilities"]["sessions"], true,
        "{initialized}"
    );
    let session = create_session(server, connection, 2, "session");
    let session = session["result"]["session"]["sessionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let thread = create_thread(server, connection, 3, "thread", &session, 0);
    let thread = thread["result"]["value"]["threadId"]
        .as_str()
        .unwrap()
        .to_owned();
    (session, thread)
}

fn enqueue(
    server: &crate::AppServer,
    connection: &mut crate::ConnectionState,
    session: &str,
    thread: &str,
) -> queue::QueuedMessage {
    let response = call(
        server,
        connection,
        json!({"jsonrpc":"2.0","id":4,"method":"queue/enqueue","params":{
            "commandId":"queued-one","sessionId":session,"threadId":thread,"input":[{"type":"text","text":"queued hello"}],
            "toolMode":"direct","approvalMode": zeta_protocol::ApprovalMode::default()
        }}),
    );
    serde_json::from_value(response["result"].clone()).expect(&response.to_string())
}

#[test]
fn queue_is_shared_across_connections_and_uses_the_existing_turn_receipt() {
    let root = tempfile::tempdir().unwrap();
    let (server, _) = queued_server(root.path());
    let mut first = server.connection();
    let (session, thread) = create(&server, &mut first);
    let message = enqueue(&server, &mut first, &session, &thread);
    let mut second = server.connection();
    initialize(&server, &mut second);
    let list = call(
        &server,
        &mut second,
        json!({"jsonrpc":"2.0","id":2,"method":"queue/list","params":{"sessionId":session,"threadId":thread}}),
    );
    assert_eq!(list["result"]["messages"], json!([message]));
    let items = call(
        &server,
        &mut second,
        json!({"jsonrpc":"2.0","id":3,"method":"extension/items/list","params":{"sessionId":session,"threadId":thread}}),
    );
    assert_eq!(items["result"]["items"][0]["body"], "queued hello");
    let queue::Delivery::Started(turn) = server.deliver_queued_message(&message).unwrap() else {
        panic!("queue was not delivered")
    };
    let queue::Delivery::Started(replayed) = server.deliver_queued_message(&message).unwrap()
    else {
        panic!("receipt was not replayed")
    };
    assert_eq!(turn, replayed);
    super::wait_for_latest_turn(&server, &thread, zeta_protocol::TurnStatus::Completed);
    assert_eq!(
        server
            .threads()
            .read_thread(&message.request.thread_id)
            .unwrap()
            .turns
            .len(),
        1
    );
}

#[test]
fn queue_dispatch_survives_connection_close() {
    let root = tempfile::tempdir().unwrap();
    let (server, store) = queued_server(root.path());
    let mut connection = server.connection();
    let (session, thread) = create(&server, &mut connection);
    let message = enqueue(&server, &mut connection, &session, &thread);
    server.close_connection(connection);
    let server = Arc::new(server);
    let runtime = server.start_queue().unwrap().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let current = store.get(&message.request.command_id).unwrap().unwrap();
        if current.status == queue::QueueStatus::Started {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "queue did not start: {current:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    super::wait_for_latest_turn(&server, &thread, zeta_protocol::TurnStatus::Completed);
    drop(runtime);
}

#[test]
fn diagnostics_and_feedback_exclude_rpc_content_and_bind_the_owner() {
    let server = server();
    let mut first = server.connection();
    initialize(&server, &mut first);
    call(
        &server,
        &mut first,
        json!({"jsonrpc":"2.0","id":2,"method":"not-a-method","params":{"prompt":"private-user-content","authorization":"secret-token"}}),
    );
    let prepared = call(
        &server,
        &mut first,
        json!({"jsonrpc":"2.0","id":3,"method":"feedback/prepare","params":{"endpoint":"https://feedback.example.test/submit"}}),
    );
    let content = prepared["result"]["content"].as_str().unwrap();
    assert!(!content.contains("private-user-content"));
    assert!(!content.contains("secret-token"));
    assert!(content.contains("activities"));
    let mut second = server.connection();
    initialize(&server, &mut second);
    let rejected = call(
        &server,
        &mut second,
        json!({"jsonrpc":"2.0","id":2,"method":"feedback/upload","params":{"operationId":"upload","digest":prepared["result"]["digest"]}}),
    );
    assert!(rejected.get("error").is_some());
    server.close_connection(first);
}

#[test]
fn persistent_queue_recovers_after_backend_restart_without_duplicate_turns() {
    let profile = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let open = || {
        crate::open_local_app_server(
            crate::LocalAppServerOptions::new(profile.path())
                .with_dir_root(directory.path())
                .with_agent_model_service(Arc::new(crate::local::ProviderModelService::new(
                    Arc::new(super::EchoModel),
                ))),
        )
        .unwrap()
    };
    let first = open();
    let mut connection = first.connection();
    let (session, thread) = create(&first, &mut connection);
    let message = enqueue(&first, &mut connection, &session, &thread);
    first.close_connection(connection);
    drop(first);
    let second = Arc::new(open());
    let runtime = second.start_queue().unwrap().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = second
            .threads()
            .read_thread(&message.request.thread_id)
            .unwrap();
        if snapshot
            .turns
            .last()
            .is_some_and(|turn| turn.status == zeta_protocol::TurnStatus::Completed)
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "restarted backend did not execute the queue"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(runtime);
    drop(second);
    let third = open();
    let queue::Delivery::Started(_) = third.deliver_queued_message(&message).unwrap() else {
        panic!("expected original receipt")
    };
    assert_eq!(
        third
            .threads()
            .read_thread(&message.request.thread_id)
            .unwrap()
            .turns
            .len(),
        1
    );
}

#[test]
fn feature_overrides_gate_turns_and_revoking_analytics_clears_observations() {
    let root = tempfile::tempdir().unwrap();
    let config =
        Arc::new(zeta_config::ConfigStore::open(root.path().join("config.sqlite")).unwrap());
    let server = server().with_config_store(config);
    let mut connection = server.connection();
    let (session, thread) = create(&server, &mut connection);
    let updated = call(
        &server,
        &mut connection,
        json!({"jsonrpc":"2.0","id":4,"method":"config/update","params":{
            "commandId":"features","expectedRevision":0,"features":{"codeMode":false,"analytics":true}
        }}),
    );
    assert!(updated.get("error").is_none(), "{updated}");
    let thread_id = zeta_protocol::ThreadId::new(&thread).unwrap();
    let sequence = server.threads().read_thread(&thread_id).unwrap().sequence;
    let request = |id, command, mode| {
        json!({"jsonrpc":"2.0","id":id,"method":"session/request","params":{
            "commandId":command,"sessionId":session,"request":{"type":"startTurn","threadId":thread,"expectedSequence":sequence,
            "approvalMode":zeta_protocol::ApprovalMode::default(),"toolMode":mode,"input":[{"type":"text","text":"hello"}]}
        }})
    };
    let disabled = call(&server, &mut connection, request(5, "disabled", "codeMode"));
    assert_eq!(
        disabled["error"]["message"], "FeatureDisabled",
        "{disabled}"
    );
    let started = call(&server, &mut connection, request(6, "ordinary", "direct"));
    assert!(started.get("error").is_none(), "{started}");
    super::wait_for_latest_turn(&server, &thread, zeta_protocol::TurnStatus::Completed);
    let diagnostics = call(
        &server,
        &mut connection,
        json!({"jsonrpc":"2.0","id":7,"method":"diagnostics/read","params":{}}),
    );
    assert_eq!(
        diagnostics["result"]["usage"]["counts"]["turnStarted"], 1,
        "{diagnostics}"
    );
    let revoked = call(
        &server,
        &mut connection,
        json!({"jsonrpc":"2.0","id":8,"method":"config/update","params":{
            "commandId":"revoke","expectedRevision":1,"features":null
        }}),
    );
    assert!(revoked.get("error").is_none(), "{revoked}");
    let diagnostics = call(
        &server,
        &mut connection,
        json!({"jsonrpc":"2.0","id":9,"method":"diagnostics/read","params":{}}),
    );
    assert_eq!(
        diagnostics["result"]["usage"],
        json!({"enabled":false,"counts":{}})
    );
}

#[test]
fn memories_are_profile_owned_retry_safe_searchable_and_deletable() {
    let root = tempfile::tempdir().unwrap();
    let server = server()
        .with_local_memories(&root.path().join("state.sqlite"))
        .unwrap();

    let mut regular = server.connection();
    initialize(&server, &mut regular);
    let denied = call(
        &server,
        &mut regular,
        json!({"jsonrpc":"2.0","id":2,"method":"memory/list","params":{"scope":{"type":"profile"}}}),
    );
    assert_eq!(denied["error"]["message"], "PermissionRequired");

    let mut host = server.product_host_connection();
    initialize(&server, &mut host);
    let mut observer = server.product_host_connection();
    initialize(&server, &mut observer);
    let notifications = server.connection_notifications(&observer);

    let add = json!({
        "commandId":"memory-add-rpc",
        "memoryId":"memory-rpc",
        "scope":{"type":"profile"},
        "title":"Preferred editor",
        "body":"Use Zeta for Rust work."
    });
    let added = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":3,"method":"memory/add","params":add.clone()}),
    );
    assert_eq!(added["result"]["disposition"], "committed", "{added}");
    assert_eq!(added["result"]["memory"]["revision"], 1);

    let changed = notifications.drain();
    assert_eq!(changed.len(), 1);
    let changed: serde_json::Value = serde_json::from_str(&changed[0]).unwrap();
    assert_eq!(changed["method"], "memory/changed");
    assert_eq!(changed["params"]["catalogRevision"], 1);

    let replayed = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":4,"method":"memory/add","params":add.clone()}),
    );
    assert_eq!(replayed["result"]["disposition"], "replayed");
    assert!(notifications.drain().is_empty());

    let listed = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":5,"method":"memory/list","params":{"scope":{"type":"profile"},"limit":10}}),
    );
    assert_eq!(listed["result"]["memories"][0]["memoryId"], "memory-rpc");
    assert!(listed["result"]["memories"][0].get("body").is_none());

    let searched = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":6,"method":"memory/search","params":{"scope":{"type":"profile"},"query":"ZETA"}}),
    );
    assert_eq!(searched["result"]["matches"][0]["memoryId"], "memory-rpc");
    assert!(
        searched["result"]["matches"][0]["excerpt"]
            .as_str()
            .unwrap()
            .contains("Zeta")
    );

    let read = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":7,"method":"memory/read","params":{"scope":{"type":"profile"},"memoryId":"memory-rpc"}}),
    );
    assert_eq!(read["result"]["body"], "Use Zeta for Rust work.");

    let deleted = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":8,"method":"memory/delete","params":{
            "commandId":"memory-delete-rpc","scope":{"type":"profile"},"memoryId":"memory-rpc","expectedRevision":1
        }}),
    );
    assert_eq!(deleted["result"]["disposition"], "committed");
    assert_eq!(notifications.drain().len(), 1);

    let removed = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":9,"method":"memory/read","params":{"scope":{"type":"profile"},"memoryId":"memory-rpc"}}),
    );
    assert_eq!(removed["error"]["message"], "MemoryNotFound");
    let old_add = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":10,"method":"memory/add","params":add}),
    );
    assert_eq!(old_add["error"]["message"], "MemoryNotFound");
}

#[test]
fn memory_policy_rpc_controls_actual_turn_context_and_citation_access() {
    let root = tempfile::tempdir().unwrap();
    let model = Arc::new(super::RecordingModel::default());
    let server = super::server_with_model(model.clone())
        .with_local_memories(&root.path().join("state.sqlite"))
        .unwrap()
        .with_tool_service(
            Arc::new(zeta_core::NoTools),
            Arc::new(super::ShellTestPolicy),
        );
    let mut host = server.product_host_connection();
    let (session, thread) = create(&server, &mut host);
    let add = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":10,"method":"memory/add","params":{
            "commandId":"add","memoryId":"preference","scope":{"type":"profile"},"title":"Rust choice","body":"Rust memory evidence: use the checked project conventions."
        }}),
    );
    assert!(add.get("error").is_none(), "{add}");
    let start = |host: &mut crate::ConnectionState, id: u64| {
        let sequence = server
            .threads()
            .read_thread(&zeta_protocol::ThreadId::new(&thread).unwrap())
            .unwrap()
            .sequence;
        let started = call(
            &server,
            host,
            json!({"jsonrpc":"2.0","id":id,"method":"session/request","params":{
                "commandId":format!("turn-{id}"),"sessionId":session,"request":{"type":"startTurn","threadId":thread,
                    "expectedSequence":sequence,"input":[{"type":"text","text":"Work on Rust"}],"toolMode":"direct"}
            }}),
        );
        assert!(started.get("error").is_none(), "{started}");
        super::wait_for_latest_turn(&server, &thread, zeta_protocol::TurnStatus::Completed);
    };
    start(&mut host, 11);
    let text = |request: &zeta_protocol::ModelRequest| serde_json::to_string(request).unwrap();
    assert!(!text(&model.requests()[0]).contains("Rust memory evidence"));
    let policy = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":12,"method":"memory/policy/read","params":{"scope":{"type":"profile"}}}),
    );
    assert_eq!(policy["result"]["automaticRead"], "disabled");
    let policy_request_id = std::cell::Cell::new(100_u64);
    let update = |host: &mut crate::ConnectionState, command: &str, revision: u64, mode: &str| {
        let id = policy_request_id.get();
        policy_request_id.set(id + 1);
        call(
            &server,
            host,
            json!({"jsonrpc":"2.0","id":id,"method":"memory/policy/update","params":{"commandId":command,"scope":{"type":"profile"},"expectedRevision":revision,"automaticRead":mode}}),
        )
    };
    let enabled = update(&mut host, "enable", 0, "firstInvocation");
    assert_eq!(enabled["result"]["policy"]["revision"], 1, "{enabled}");
    start(&mut host, 14);
    let requests = model.requests();
    assert!(text(&requests[1]).contains("Rust memory evidence"));
    assert!(
        !requests[1]
            .instructions
            .as_deref()
            .unwrap_or_default()
            .contains("Rust memory evidence")
    );
    let search = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":15,"method":"memory/search","params":{"scope":{"type":"profile"},"query":"Rust"}}),
    );
    let citation = search["result"]["matches"][0]["citation"].clone();
    let read = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":16,"method":"memory/citation/read","params":{"citation":citation.clone()}}),
    );
    assert_eq!(
        read["result"]["body"],
        search["result"]["matches"][0]["excerpt"]
    );
    let mut regular = server.connection();
    initialize(&server, &mut regular);
    for (index, (method, params)) in [
        ("memory/citation/read", json!({"citation":citation})),
        ("memory/policy/read", json!({"scope":{"type":"profile"}})),
        (
            "memory/policy/update",
            json!({"commandId":"unauthorized","scope":{"type":"profile"},"expectedRevision":1,"automaticRead":"disabled"}),
        ),
    ].into_iter().enumerate() {
        let denied = call(
            &server,
            &mut regular,
            json!({"jsonrpc":"2.0","id":17 + index,"method":method,"params":params}),
        );
        assert_eq!(denied["error"]["message"], "PermissionRequired", "{denied}");
    }
    assert_eq!(
        update(&mut host, "disable", 1, "disabled")["result"]["policy"]["revision"],
        2
    );
    start(&mut host, 18);
    assert!(!text(&model.requests()[2]).contains("Rust memory evidence"));
    assert_eq!(
        update(&mut host, "enable", 0, "firstInvocation")["result"]["disposition"],
        "replayed"
    );
    start(&mut host, 19);
    assert!(!text(&model.requests()[3]).contains("Rust memory evidence"));
    assert_eq!(
        update(&mut host, "reenable", 2, "firstInvocation")["result"]["policy"]["revision"],
        3
    );
    start(&mut host, 20);
    assert!(text(&model.requests()[4]).contains("Rust memory evidence"));
    let deleted = call(
        &server,
        &mut host,
        json!({"jsonrpc":"2.0","id":21,"method":"memory/delete","params":{
            "commandId":"delete","scope":{"type":"profile"},"memoryId":"preference","expectedRevision":1
        }}),
    );
    assert_eq!(deleted["result"]["disposition"], "committed", "{deleted}");
    start(&mut host, 22);
    assert!(!text(&model.requests()[5]).contains("Rust memory evidence"));
    let snapshot = server
        .threads()
        .read_thread(&zeta_protocol::ThreadId::new(&thread).unwrap())
        .unwrap();
    assert!(!format!("{:?}", snapshot.items).contains("Rust memory evidence"));
}

struct MemoryToolModel {
    reference: String,
    requests: std::sync::Mutex<Vec<zeta_protocol::ModelRequest>>,
}

impl zeta_core::ModelService for MemoryToolModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<zeta_protocol::ModelResponse, zeta_core::CoreError> {
        let mut requests = self.requests.lock().unwrap();
        let step = requests.len();
        requests.push(request.clone());
        let output = match step {
            0 => zeta_protocol::ResponseItem::ToolCall(zeta_protocol::ToolCall {
                id: zeta_protocol::ToolCallId::new("search-memory").unwrap(),
                name: zeta_protocol::ToolName::new("memories-search").unwrap(),
                arguments: json!({"query":"Rust"}),
            }),
            1 => zeta_protocol::ResponseItem::ToolCall(zeta_protocol::ToolCall {
                id: zeta_protocol::ToolCallId::new("read-memory").unwrap(),
                name: zeta_protocol::ToolName::new("memories-read").unwrap(),
                arguments: json!({"reference":self.reference}),
            }),
            _ => zeta_protocol::ResponseItem::Text("done".into()),
        };
        Ok(zeta_protocol::ModelResponse {
            output: vec![output],
            usage: None,
            billing: None,
            stop_reason: if step < 2 {
                zeta_protocol::StopReason::ToolUse
            } else {
                zeta_protocol::StopReason::Completed
            },
        })
    }
}

#[test]
fn memory_extension_tools_execute_in_turns_and_survive_host_composition_order() {
    for memories_first in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state.sqlite");
        let skills = root.path().join("skills");
        std::fs::create_dir(&skills).unwrap();
        let body = "Rust persisted decision";
        let citation = memories::MemoryCitation {
            memory_id: memories::MemoryId::new("decision").unwrap(),
            scope: memories::MemoryScope::Profile,
            revision: 1,
            start_byte: 0,
            end_byte: body.len() as u32,
        };
        let model = Arc::new(MemoryToolModel {
            reference: citation.reference().unwrap(),
            requests: std::sync::Mutex::new(Vec::new()),
        });
        let mut server = super::server_with_model(model.clone());
        if memories_first {
            server = server.with_local_memories(&path).unwrap();
        }
        server = server
            .with_skill_runtime(
                zeta_skills_extension::BuiltInSkillSource::Root(skills),
                Arc::new(super::EmptySkillConfig),
                None,
            )
            .unwrap();
        if !memories_first {
            server = server.with_local_memories(&path).unwrap();
        }
        // Installing Projects after Memories must replace both the context contributor and tools.
        let server = server.with_local_projects(&path).unwrap();
        let mut host = server.product_host_connection();
        let (session, thread) = create(&server, &mut host);
        let added = call(
            &server,
            &mut host,
            json!({"jsonrpc":"2.0","id":10,"method":"memory/add","params":{
                "commandId":"add","memoryId":"decision","scope":{"type":"profile"},"title":"Rust","body":body
            }}),
        );
        assert!(added.get("error").is_none(), "{added}");
        let enabled = call(
            &server,
            &mut host,
            json!({"jsonrpc":"2.0","id":11,"method":"memory/policy/update","params":{
                "commandId":"enable","scope":{"type":"profile"},"expectedRevision":0,"automaticRead":"firstInvocation"
            }}),
        );
        assert!(enabled.get("error").is_none(), "{enabled}");
        let sequence = server
            .threads()
            .read_thread(&zeta_protocol::ThreadId::new(&thread).unwrap())
            .unwrap()
            .sequence;
        let started = call(
            &server,
            &mut host,
            json!({"jsonrpc":"2.0","id":12,"method":"session/request","params":{
                "commandId":"turn","sessionId":session,"request":{"type":"startTurn","threadId":thread,
                "expectedSequence":sequence,"input":[{"type":"text","text":"Rust"}],"toolMode":"direct"}
            }}),
        );
        assert!(started.get("error").is_none(), "{started}");
        super::wait_for_latest_turn(&server, &thread, zeta_protocol::TurnStatus::Completed);
        let requests = model.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        let first = serde_json::to_string(&requests[0]).unwrap();
        for name in ["memories-search", "memories-read", "skills-read"] {
            assert!(first.contains(name), "missing {name}");
        }
        assert!(first.contains("context_evidence"));
        assert!(
            !requests[0]
                .instructions
                .as_deref()
                .unwrap_or_default()
                .contains(body)
        );
        let second = serde_json::to_string(&requests[1]).unwrap();
        let third = serde_json::to_string(&requests[2]).unwrap();
        assert!(!second.contains("context_evidence"));
        assert!(
            second.contains(&citation.reference().unwrap()),
            "search did not return its reference: {second}"
        );
        assert!(third.contains(body));
        let snapshot = server
            .threads()
            .read_thread(&zeta_protocol::ThreadId::new(&thread).unwrap())
            .unwrap();
        let completed = snapshot
            .items
            .iter()
            .filter_map(|item| match item {
                zeta_protocol::ThreadItem::ToolResult { text, is_error, .. } => {
                    Some((text, is_error))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(completed.len(), 2);
        assert!(
            completed
                .iter()
                .all(|(text, is_error)| !**is_error && text.contains(body))
        );
    }
}
