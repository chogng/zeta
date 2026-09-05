use super::AppServer;
use super::ConnectionState;
use crate::local::ProviderModelService;
use std::sync::Arc;
use zeta_core::InMemoryThreadStore;
use zeta_core::StartThreadRequest;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;
use zeta_protocol::CommandId;

#[test]
fn work_run_rpc_checks_host_authority_and_real_thread_topology() {
    let fixture = Fixture::new();
    let first = fixture.start_root("first-root");
    let second = fixture.start_root("second-root");
    let mut renderer = fixture.server.connection();
    initialize(&fixture.server, &mut renderer, false);
    let denied = call(
        &fixture.server,
        &mut renderer,
        2,
        "workRun/list",
        serde_json::json!({}),
    );
    assert_eq!(denied["error"]["message"], "PermissionRequired");

    let mut self_promoted = fixture.server.connection();
    let rejected = call(
        &fixture.server,
        &mut self_promoted,
        3,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "self-promoted", "version": "1"},
            "capabilities": {"workCoordinationHost": {"version": 1}}
        }),
    );
    assert_eq!(rejected["error"]["message"], "PermissionRequired");

    let mut host = fixture.server.product_host_connection();
    initialize(&fixture.server, &mut host, true);
    let created = call(
        &fixture.server,
        &mut host,
        4,
        "workRun/create",
        serde_json::json!({
            "commandId": "create-work-run-rpc",
            "workRunId": "work-run-rpc",
            "rootSessionId": first.session_id,
            "rootThreadId": first.thread_id,
            "objective": "coordinate two independent sessions",
            "acceptanceConditions": ["topology is exact"],
            "exclusions": []
        }),
    );
    assert_eq!(created["result"]["disposition"], "committed");
    assert_eq!(created["result"]["workRun"]["revision"], 1);

    let fake_child = call(
        &fixture.server,
        &mut host,
        5,
        "workRun/participant/add",
        serde_json::json!({
            "commandId": "fake-child",
            "workRunId": "work-run-rpc",
            "expectedRevision": 1,
            "sessionId": second.session_id,
            "threadId": second.thread_id,
            "relation": {
                "type": "delegated",
                "parentThreadId": first.thread_id,
                "delegationId": "forged-delegation"
            }
        }),
    );
    assert_eq!(fake_child["error"]["message"], "InvalidParams");

    let added = call(
        &fixture.server,
        &mut host,
        6,
        "workRun/participant/add",
        serde_json::json!({
            "commandId": "add-second-root",
            "workRunId": "work-run-rpc",
            "expectedRevision": 1,
            "sessionId": second.session_id,
            "threadId": second.thread_id,
            "relation": {"type": "root"}
        }),
    );
    assert_eq!(added["result"]["workRun"]["revision"], 2);
    assert_eq!(added["result"]["workRun"]["topologyRevision"], 2);
    assert_eq!(
        added["result"]["workRun"]["participants"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let listed = call(
        &fixture.server,
        &mut host,
        7,
        "workRun/list",
        serde_json::json!({}),
    );
    assert_eq!(listed["result"]["workRuns"][0]["sessionCount"], 2);

    let view = call(
        &fixture.server,
        &mut host,
        8,
        "workRun/view/read",
        serde_json::json!({"workRunId": "work-run-rpc"}),
    );
    assert_eq!(view["result"]["collaborationMode"], "multiSession");
    assert_eq!(view["result"]["sessionTrees"].as_array().unwrap().len(), 2);
    assert!(
        view["result"]["sessionTrees"]
            .as_array()
            .unwrap()
            .iter()
            .all(|session| session["agentTree"]["roots"].as_array().unwrap().len() == 1)
    );
}

#[test]
fn work_run_rpc_replays_without_emitting_a_second_change() {
    let fixture = Fixture::new();
    let root = fixture.start_root("replay-root");
    let mut host = fixture.server.product_host_connection();
    initialize(&fixture.server, &mut host, true);
    let notifications = fixture.server.connection_notifications(&host);
    let params = serde_json::json!({
        "commandId": "replay-create",
        "workRunId": "replay-run",
        "rootSessionId": root.session_id,
        "rootThreadId": root.thread_id,
        "objective": "replay exactly",
        "acceptanceConditions": ["one durable result"],
        "exclusions": []
    });
    let committed = call(
        &fixture.server,
        &mut host,
        2,
        "workRun/create",
        params.clone(),
    );
    assert_eq!(committed["result"]["disposition"], "committed");
    let first = notifications.drain();
    assert_eq!(first.len(), 1);
    let first: serde_json::Value = serde_json::from_str(&first[0]).unwrap();
    assert_eq!(first["method"], "workRun/changed");

    let replayed = call(&fixture.server, &mut host, 3, "workRun/create", params);
    assert_eq!(replayed["result"]["disposition"], "replayed");
    assert!(notifications.drain().is_empty());
}

struct Fixture {
    _directory: tempfile::TempDir,
    server: AppServer,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let threads = Arc::new(ThreadController::with_store(Arc::new(
            InMemoryThreadStore::default(),
        )));
        let server = AppServer::new(
            Arc::clone(&threads),
            Arc::new(ProviderModelService::new(Arc::new(EchoModel))),
        )
        .with_local_work_coordination(&directory.path().join("state.sqlite3"))
        .unwrap();
        Self {
            _directory: directory,
            server,
        }
    }

    fn start_root(&self, command_id: &str) -> zeta_core::ThreadSnapshot {
        self.server
            .start_thread(StartThreadRequest {
                command_id: CommandId::new(command_id).unwrap(),
                title: command_id.into(),
            })
            .unwrap()
    }
}

fn initialize(server: &AppServer, connection: &mut ConnectionState, host: bool) {
    let capabilities = if host {
        serde_json::json!({"workCoordinationHost": {"version": 1}})
    } else {
        serde_json::json!({})
    };
    let initialized = call(
        server,
        connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "work-run-test", "version": "1"},
            "capabilities": capabilities
        }),
    );
    assert!(initialized.get("result").is_some());
}

fn call(
    server: &AppServer,
    connection: &mut ConnectionState,
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
