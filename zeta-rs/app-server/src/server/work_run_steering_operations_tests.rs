use super::AppServer;
use super::ConnectionState;
use crate::local::ProviderModelService;
use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;
use zeta_core::InMemoryThreadStore;
use zeta_core::StartThreadRequest;
use zeta_core::ThreadController;
use zeta_file_access::DirId;
use zeta_file_access::EnvId;
use zeta_model_provider::EchoModel;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_work_coordination::AuthorizationSnapshotRef;
use zeta_work_coordination::ManagedRootBinding;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::ValidationProfileRef;
use zeta_work_coordination::WorkContractDraft;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkStartMode;
use zeta_work_coordination::root_checkpoint_digest;

#[test]
fn work_run_decisions_reject_untrusted_connections_without_mutating_state() {
    let fixture = Fixture::new();
    let root = fixture.start_root("authority-root");
    let mut host = fixture.server.product_host_connection();
    initialize(&fixture.server, &mut host, true);
    create_run(&fixture.server, &mut host, 2, "authority-run", &root);

    let mut renderer = fixture.server.connection();
    initialize(&fixture.server, &mut renderer, false);
    let denied = call(
        &fixture.server,
        &mut renderer,
        2,
        "workRun/decision/record",
        serde_json::json!({
            "commandId": "forged-decision-command",
            "workRunId": "authority-run",
            "expectedRevision": 1,
            "decisionId": "forged-decision",
            "authority": "user",
            "scope": "architecture",
            "statement": "trust the Agent claim"
        }),
    );
    assert_eq!(denied["error"]["message"], "PermissionRequired");

    let read = call(
        &fixture.server,
        &mut host,
        3,
        "workRun/read",
        serde_json::json!({"workRunId": "authority-run"}),
    );
    assert_eq!(read["result"]["workRun"]["revision"], 1);
    assert_eq!(
        read["result"]["workRun"]["decisions"],
        serde_json::json!([])
    );
}

#[test]
fn scope_expansion_stops_only_the_exact_attempt_and_replays_once() {
    let fixture = Fixture::new();
    let root = fixture.start_root("scope-root");
    let mut host = fixture.server.product_host_connection();
    initialize(&fixture.server, &mut host, true);
    create_run(&fixture.server, &mut host, 2, "scope-run", &root);
    let run_id = WorkRunId::new("scope-run").unwrap();
    let prepared = fixture.seed_writing_attempt(
        &run_id,
        1,
        "scope-attempt",
        "scope-contract",
        &root.thread_id,
        1,
    );

    let empty = call(
        &fixture.server,
        &mut host,
        3,
        "workRun/attempt/scopeExpansion/request",
        serde_json::json!({
            "commandId": "empty-scope-expansion",
            "workRunId": "scope-run",
            "expectedRevision": prepared.revision,
            "attemptId": "scope-attempt",
            "evidence": []
        }),
    );
    assert_eq!(empty["error"]["message"], "WorkCoordinationOperationFailed");

    let params = serde_json::json!({
        "commandId": "request-scope-expansion",
        "workRunId": "scope-run",
        "expectedRevision": prepared.revision,
        "attemptId": "scope-attempt",
        "evidence": ["the public schema must change"]
    });
    let expanded = call(
        &fixture.server,
        &mut host,
        4,
        "workRun/attempt/scopeExpansion/request",
        params.clone(),
    );
    assert_eq!(expanded["result"]["disposition"], "committed");
    assert_eq!(
        expanded["result"]["workRun"]["revision"],
        prepared.revision + 1
    );
    let attempt = find_by_id(
        &expanded["result"]["workRun"]["attempts"],
        "attemptId",
        "scope-attempt",
    );
    assert_eq!(attempt["executionStatus"], "interrupted");
    assert_eq!(attempt["coordinationStatus"], "expansionRequested");
    assert_eq!(
        attempt["scopeExpansionEvidence"],
        serde_json::json!(["the public schema must change"])
    );

    let replayed = call(
        &fixture.server,
        &mut host,
        5,
        "workRun/attempt/scopeExpansion/request",
        params,
    );
    assert_eq!(replayed["result"]["disposition"], "replayed");
    assert_eq!(
        replayed["result"]["workRun"]["revision"],
        prepared.revision + 1
    );

    let repeated = call(
        &fixture.server,
        &mut host,
        6,
        "workRun/attempt/scopeExpansion/request",
        serde_json::json!({
            "commandId": "repeat-scope-expansion",
            "workRunId": "scope-run",
            "expectedRevision": prepared.revision + 1,
            "attemptId": "scope-attempt",
            "evidence": ["try to continue the old attempt"]
        }),
    );
    assert_eq!(
        repeated["error"]["message"],
        "WorkCoordinationOperationFailed"
    );
}

#[test]
fn conflicts_require_exact_versions_and_resolutions_make_old_attempts_stale() {
    let fixture = Fixture::new();
    let first = fixture.start_root("conflict-first");
    let second = fixture.start_root("conflict-second");
    let mut host = fixture.server.product_host_connection();
    initialize(&fixture.server, &mut host, true);
    create_run(&fixture.server, &mut host, 2, "conflict-run", &first);
    let added = call(
        &fixture.server,
        &mut host,
        3,
        "workRun/participant/add",
        serde_json::json!({
            "commandId": "add-conflict-peer",
            "workRunId": "conflict-run",
            "expectedRevision": 1,
            "sessionId": second.session_id,
            "threadId": second.thread_id,
            "relation": {"type": "root"}
        }),
    );
    assert_eq!(added["result"]["workRun"]["revision"], 2);
    let run_id = WorkRunId::new("conflict-run").unwrap();
    let first_attempt = fixture.seed_writing_attempt(
        &run_id,
        2,
        "first-attempt",
        "first-contract",
        &first.thread_id,
        2,
    );
    let prepared = fixture.seed_writing_attempt(
        &run_id,
        first_attempt.revision,
        "second-attempt",
        "second-contract",
        &second.thread_id,
        2,
    );

    let decision_params = serde_json::json!({
        "commandId": "record-conflict-policy",
        "workRunId": "conflict-run",
        "expectedRevision": prepared.revision,
        "decisionId": "single-schema-writer",
        "authority": "authorized-work-owner",
        "scope": "shared protocol schema",
        "statement": "the first replacement Attempt owns the schema migration"
    });
    let decided = call(
        &fixture.server,
        &mut host,
        4,
        "workRun/decision/record",
        decision_params.clone(),
    );
    assert_eq!(decided["result"]["disposition"], "committed");
    assert_eq!(
        decided["result"]["workRun"]["revision"],
        prepared.revision + 1
    );
    let replayed = call(
        &fixture.server,
        &mut host,
        5,
        "workRun/decision/record",
        decision_params,
    );
    assert_eq!(replayed["result"]["disposition"], "replayed");

    let conflicting_replay = call(
        &fixture.server,
        &mut host,
        6,
        "workRun/decision/record",
        serde_json::json!({
            "commandId": "record-conflict-policy",
            "workRunId": "conflict-run",
            "expectedRevision": prepared.revision,
            "decisionId": "single-schema-writer",
            "authority": "authorized-work-owner",
            "scope": "shared protocol schema",
            "statement": "a different decision under the same command identity"
        }),
    );
    assert_eq!(conflicting_replay["error"]["message"], "CommandConflict");

    let stale = call(
        &fixture.server,
        &mut host,
        7,
        "workRun/conflict/record",
        conflict_params(prepared.revision, "stale-conflict", false),
    );
    assert_eq!(
        stale["error"]["message"],
        "WorkCoordinationRevisionConflict"
    );

    let duplicate = call(
        &fixture.server,
        &mut host,
        8,
        "workRun/conflict/record",
        conflict_params(prepared.revision + 1, "duplicate-conflict", true),
    );
    assert_eq!(
        duplicate["error"]["message"],
        "WorkCoordinationOperationFailed"
    );

    let conflicted = call(
        &fixture.server,
        &mut host,
        9,
        "workRun/conflict/record",
        conflict_params(prepared.revision + 1, "shared-schema-conflict", false),
    );
    assert_eq!(conflicted["result"]["disposition"], "committed");
    assert_eq!(
        conflicted["result"]["workRun"]["revision"],
        prepared.revision + 2
    );
    for attempt_id in ["first-attempt", "second-attempt"] {
        let attempt = find_by_id(
            &conflicted["result"]["workRun"]["attempts"],
            "attemptId",
            attempt_id,
        );
        assert_eq!(attempt["executionStatus"], "interrupted");
        assert_eq!(attempt["coordinationStatus"], "conflict");
    }

    let unknown_decision = call(
        &fixture.server,
        &mut host,
        10,
        "workRun/conflict/resolve",
        serde_json::json!({
            "commandId": "resolve-with-unknown-decision",
            "workRunId": "conflict-run",
            "expectedRevision": prepared.revision + 2,
            "conflictId": "shared-schema-conflict",
            "decisionId": "unknown-decision"
        }),
    );
    assert_eq!(
        unknown_decision["error"]["message"],
        "WorkCoordinationNotFound"
    );

    let resolved = call(
        &fixture.server,
        &mut host,
        11,
        "workRun/conflict/resolve",
        serde_json::json!({
            "commandId": "resolve-shared-schema-conflict",
            "workRunId": "conflict-run",
            "expectedRevision": prepared.revision + 2,
            "conflictId": "shared-schema-conflict",
            "decisionId": "single-schema-writer"
        }),
    );
    assert_eq!(
        resolved["result"]["workRun"]["revision"],
        prepared.revision + 3
    );
    let conflict = find_by_id(
        &resolved["result"]["workRun"]["conflicts"],
        "conflictId",
        "shared-schema-conflict",
    );
    assert_eq!(conflict["status"], "resolved");
    assert_eq!(conflict["resolutionDecisionId"], "single-schema-writer");
    for attempt_id in ["first-attempt", "second-attempt"] {
        let attempt = find_by_id(
            &resolved["result"]["workRun"]["attempts"],
            "attemptId",
            attempt_id,
        );
        assert_eq!(attempt["coordinationStatus"], "stale");
        assert_eq!(attempt["verificationStatus"], "stale");
    }
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

    fn seed_writing_attempt(
        &self,
        work_run_id: &WorkRunId,
        expected_revision: u64,
        attempt_id: &str,
        contract_id: &str,
        thread_id: &ThreadId,
        topology_revision: u64,
    ) -> WorkRun {
        let contract_id = WorkContractId::new(contract_id).unwrap();
        let attempt_id = WorkAttemptId::new(attempt_id).unwrap();
        let contracted = self.apply_state(
            work_run_id,
            expected_revision,
            &format!("create-{contract_id}"),
            WorkRunCommand::CreateContract {
                contract: contract(&contract_id, thread_id, topology_revision),
            },
        );
        let attempted = self.apply_state(
            work_run_id,
            contracted.revision,
            &format!("create-{attempt_id}"),
            WorkRunCommand::CreateAttempt {
                attempt_id: attempt_id.clone(),
                contract: WorkContractRef {
                    contract_id,
                    revision: 1,
                },
                participant_thread_id: thread_id.clone(),
            },
        );
        let root = attempted.attempts[&attempt_id].roots[0].clone();
        let ready = self.apply_state(
            work_run_id,
            attempted.revision,
            &format!("ready-{attempt_id}"),
            WorkRunCommand::RecordAttemptWorkspaceReady {
                attempt_id: attempt_id.clone(),
                roots: vec![ManagedRootBinding {
                    source_dir_id: root.dir_id.clone(),
                    managed_dir_id: dir(&format!("managed-{attempt_id}")),
                    root_checkpoint_digest: root_checkpoint_digest(&root).unwrap(),
                    binding_manifest_digest: digest(&format!("binding-{attempt_id}")),
                }],
                private_output_dir_id: dir(&format!("output-{attempt_id}")),
            },
        );
        self.apply_state(
            work_run_id,
            ready.revision,
            &format!("begin-{attempt_id}"),
            WorkRunCommand::BeginAttempt {
                attempt_id: attempt_id.clone(),
                execution_id: WorkExecutionId::new(format!("execution-{attempt_id}")).unwrap(),
                mode: WorkStartMode::Write,
            },
        )
    }

    fn apply_state(
        &self,
        work_run_id: &WorkRunId,
        expected_revision: u64,
        command_id: &str,
        command: WorkRunCommand,
    ) -> WorkRun {
        self.server
            .work_coordination
            .as_ref()
            .unwrap()
            .apply_state_for_test(WorkRunCommandRequest {
                command_id: CommandId::new(command_id).unwrap(),
                work_run_id: work_run_id.clone(),
                expected_revision,
                command,
            })
            .unwrap()
            .work_run
    }
}

fn contract(
    contract_id: &WorkContractId,
    thread_id: &ThreadId,
    topology_revision: u64,
) -> WorkContractDraft {
    let root_dir_id = dir("shared-source-root");
    WorkContractDraft {
        contract_id: contract_id.clone(),
        goal_revision: 1,
        topology_revision,
        owner_thread_id: thread_id.clone(),
        objective: "execute bounded work".into(),
        acceptance_conditions: vec!["the bounded work is complete".into()],
        exclusions: Vec::new(),
        environment_id: EnvId::local(),
        roots: vec![RootCheckpoint {
            environment_id: EnvId::local(),
            dir_id: root_dir_id.clone(),
            state: RootState::Directory {
                snapshot_id: "source-snapshot".into(),
            },
            control_resources: Vec::new(),
        }],
        primary_root_dir_id: root_dir_id,
        authorization: AuthorizationSnapshotRef {
            authority: "test-authority".into(),
            policy_revision: "policy-1".into(),
            grant_set_digest: digest("grants"),
            granted_effects_digest: digest("effects"),
        },
        decision_ids: BTreeSet::new(),
        upstream_results: Vec::new(),
        expected_scope: WorkScopeClaim::default(),
        validation_profile: ValidationProfileRef {
            name: "targeted".into(),
            content_digest: digest("validation-profile"),
        },
    }
}

fn conflict_params(expected_revision: u64, command_id: &str, duplicate: bool) -> serde_json::Value {
    let attempt_ids = if duplicate {
        serde_json::json!(["first-attempt", "first-attempt"])
    } else {
        serde_json::json!(["first-attempt", "second-attempt"])
    };
    serde_json::json!({
        "commandId": command_id,
        "workRunId": "conflict-run",
        "expectedRevision": expected_revision,
        "conflictId": "shared-schema-conflict",
        "attemptIds": attempt_ids,
        "resource": "app-server protocol schema",
        "evidence": ["both Attempts write the same public contract"]
    })
}

fn create_run(
    server: &AppServer,
    connection: &mut ConnectionState,
    id: u64,
    work_run_id: &str,
    root: &zeta_core::ThreadSnapshot,
) {
    let created = call(
        server,
        connection,
        id,
        "workRun/create",
        serde_json::json!({
            "commandId": format!("create-{work_run_id}"),
            "workRunId": work_run_id,
            "rootSessionId": root.session_id,
            "rootThreadId": root.thread_id,
            "objective": "coordinate exact work",
            "acceptanceConditions": ["coordination decisions are durable"],
            "exclusions": []
        }),
    );
    assert_eq!(created["result"]["workRun"]["revision"], 1);
}

fn find_by_id<'a>(
    values: &'a serde_json::Value,
    field: &str,
    expected: &str,
) -> &'a serde_json::Value {
    values
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value[field] == expected)
        .unwrap()
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
            "clientInfo": {"name": "work-run-steering-test", "version": "1"},
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

fn dir(seed: &str) -> DirId {
    DirId::from_str(ContentDigest::sha256(seed.as_bytes()).as_str()).unwrap()
}

fn digest(value: &str) -> ContentDigest {
    ContentDigest::sha256(value.as_bytes())
}
