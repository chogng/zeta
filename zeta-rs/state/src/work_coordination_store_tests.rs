use super::SqliteWorkRunStore;
use std::collections::BTreeSet;
use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_environment::EnvId;
use zeta_file_access::DirId;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_work_coordination::AuthorizationSnapshotRef;
use zeta_work_coordination::ExternalEffectsStatus;
use zeta_work_coordination::GitRepositoryCheckpoint;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::ManagedRootBinding;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::ValidationProfileRef;
use zeta_work_coordination::WorkCommandDisposition;
use zeta_work_coordination::WorkContractDraft;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkCoordinationError;
use zeta_work_coordination::WorkCoordinator;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkStartMode;
use zeta_work_coordination::root_checkpoint_digest;

#[test]
fn sqlite_work_runs_persist_records_and_original_command_results() {
    let path = database_path("work-runs");
    let store = Arc::new(SqliteWorkRunStore::open(&path).unwrap());
    let coordinator = WorkCoordinator::new(store);
    let create = create_request();
    let created = coordinator.apply(create.clone()).unwrap();
    assert_eq!(created.work_run.revision, 1);
    coordinator
        .apply(WorkRunCommandRequest {
            command_id: CommandId::new("add-session-b").unwrap(),
            work_run_id: create.work_run_id.clone(),
            expected_revision: 1,
            command: WorkRunCommand::AddParticipant {
                participant: root_participant("session-b", "thread-b"),
            },
        })
        .unwrap();
    drop(coordinator);

    let reopened = WorkCoordinator::new(Arc::new(SqliteWorkRunStore::open(&path).unwrap()));
    assert_eq!(reopened.read(&create.work_run_id).unwrap().revision, 2);
    assert_eq!(reopened.list().unwrap().len(), 1);
    let replay = reopened.apply(create.clone()).unwrap();
    assert_eq!(replay.disposition, WorkCommandDisposition::Replayed);
    assert_eq!(replay.work_run.revision, 1);

    let mut conflicting = create;
    conflicting.command = WorkRunCommand::Create {
        objective: "different objective".into(),
        acceptance_conditions: vec!["accepted".into()],
        exclusions: Vec::new(),
        root_participant: root_participant("session-a", "thread-a"),
    };
    assert_eq!(
        reopened.apply(conflicting),
        Err(WorkCoordinationError::CommandConflict)
    );
    drop(reopened);
    fs::remove_file(path).unwrap();
}

#[test]
fn sqlite_work_runs_reject_row_metadata_corruption() {
    let path = database_path("work-run-corruption");
    let coordinator = WorkCoordinator::new(Arc::new(SqliteWorkRunStore::open(&path).unwrap()));
    let create = create_request();
    coordinator.apply(create.clone()).unwrap();
    drop(coordinator);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE work_runs SET revision = 99 WHERE work_run_id = ?1",
            [create.work_run_id.as_str()],
        )
        .unwrap();
    drop(connection);
    assert!(
        SqliteWorkRunStore::open(&path)
            .err()
            .expect("corrupt store must not open")
            .to_string()
            .contains("metadata disagrees")
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn sqlite_writer_lease_is_global_durable_and_released_atomically() {
    let path = database_path("work-run-writer-lease");
    let coordinator = WorkCoordinator::new(Arc::new(SqliteWorkRunStore::open(&path).unwrap()));
    create_ready_attempt(
        &coordinator,
        "run-a",
        "session-a",
        "shared-thread",
        "attempt-a",
    );
    create_ready_attempt(
        &coordinator,
        "run-b",
        "session-b",
        "shared-thread",
        "attempt-b",
    );
    coordinator
        .apply(command(
            "begin-a",
            "run-a",
            4,
            WorkRunCommand::BeginAttempt {
                attempt_id: WorkAttemptId::new("attempt-a").unwrap(),
                execution_id: WorkExecutionId::new("execution-a").unwrap(),
                mode: WorkStartMode::Write,
            },
        ))
        .unwrap();
    let begin_b = command(
        "begin-b",
        "run-b",
        4,
        WorkRunCommand::BeginAttempt {
            attempt_id: WorkAttemptId::new("attempt-b").unwrap(),
            execution_id: WorkExecutionId::new("execution-b").unwrap(),
            mode: WorkStartMode::Write,
        },
    );
    assert!(matches!(
        coordinator.apply(begin_b.clone()),
        Err(WorkCoordinationError::ThreadBusy { .. })
    ));
    drop(coordinator);

    let reopened = WorkCoordinator::new(Arc::new(SqliteWorkRunStore::open(&path).unwrap()));
    assert!(matches!(
        reopened.apply(begin_b.clone()),
        Err(WorkCoordinationError::ThreadBusy { .. })
    ));
    reopened
        .apply(command(
            "seal-a",
            "run-a",
            5,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt-a").unwrap(),
                result_digest: digest("result-a"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("output-a"),
                external_effects_digest: digest("effects-a"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();
    reopened.apply(begin_b).unwrap();
    drop(reopened);
    fs::remove_file(path).unwrap();
}

fn create_request() -> WorkRunCommandRequest {
    WorkRunCommandRequest {
        command_id: CommandId::new("create-work-run").unwrap(),
        work_run_id: WorkRunId::new("work-run").unwrap(),
        expected_revision: 0,
        command: WorkRunCommand::Create {
            objective: "coordinate independent work".into(),
            acceptance_conditions: vec!["state is durable".into()],
            exclusions: Vec::new(),
            root_participant: root_participant("session-a", "thread-a"),
        },
    }
}

fn root_participant(session_id: &str, thread_id: &str) -> WorkParticipant {
    WorkParticipant {
        session_id: SessionId::new(session_id).unwrap(),
        thread_id: ThreadId::new(thread_id).unwrap(),
        relation: WorkParticipantRelation::Root,
    }
}

fn create_ready_attempt(
    coordinator: &WorkCoordinator,
    run_id: &str,
    session_id: &str,
    thread_id: &str,
    attempt_id: &str,
) {
    coordinator
        .apply(WorkRunCommandRequest {
            command_id: CommandId::new(format!("create-{run_id}")).unwrap(),
            work_run_id: WorkRunId::new(run_id).unwrap(),
            expected_revision: 0,
            command: WorkRunCommand::Create {
                objective: "coordinate one writer".into(),
                acceptance_conditions: vec!["writer is isolated".into()],
                exclusions: Vec::new(),
                root_participant: root_participant(session_id, thread_id),
            },
        })
        .unwrap();
    let contract_id = WorkContractId::new(format!("contract-{run_id}")).unwrap();
    let source_dir_id = dir(&format!("source-{run_id}"));
    let checkpoint = RootCheckpoint {
        environment_id: EnvId::local(),
        dir_id: source_dir_id.clone(),
        state: RootState::Git {
            repositories: vec![GitRepositoryCheckpoint {
                repository_id: format!("repository-{run_id}"),
                relative_path: ".".into(),
                target: GitRootTarget::Branch {
                    name: "main".into(),
                    expected_head: format!("head-{run_id}"),
                },
                baseline_tree: format!("tree-{run_id}"),
            }],
        },
        control_resources: Vec::new(),
    };
    coordinator
        .apply(command(
            &format!("contract-{run_id}"),
            run_id,
            1,
            WorkRunCommand::CreateContract {
                contract: WorkContractDraft {
                    contract_id: contract_id.clone(),
                    goal_revision: 1,
                    topology_revision: 1,
                    owner_thread_id: ThreadId::new(thread_id).unwrap(),
                    objective: "write isolated code".into(),
                    acceptance_conditions: vec!["checks pass".into()],
                    exclusions: Vec::new(),
                    environment_id: EnvId::local(),
                    roots: vec![checkpoint.clone()],
                    primary_root_dir_id: source_dir_id.clone(),
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
                        name: "test".into(),
                        content_digest: digest("validation"),
                    },
                },
            },
        ))
        .unwrap();
    let attempt_id = WorkAttemptId::new(attempt_id).unwrap();
    coordinator
        .apply(command(
            &format!("attempt-{run_id}"),
            run_id,
            2,
            WorkRunCommand::CreateAttempt {
                attempt_id: attempt_id.clone(),
                contract: WorkContractRef {
                    contract_id,
                    revision: 1,
                },
                participant_thread_id: ThreadId::new(thread_id).unwrap(),
            },
        ))
        .unwrap();
    let checkpoint_digest = root_checkpoint_digest(&checkpoint).unwrap();
    coordinator
        .apply(command(
            &format!("workspace-{run_id}"),
            run_id,
            3,
            WorkRunCommand::RecordAttemptWorkspaceReady {
                attempt_id,
                roots: vec![ManagedRootBinding {
                    source_dir_id,
                    managed_dir_id: dir(&format!("managed-{run_id}")),
                    root_checkpoint_digest: checkpoint_digest,
                    binding_manifest_digest: digest(&format!("binding-{run_id}")),
                }],
                private_output_dir_id: dir(&format!("output-{run_id}")),
            },
        ))
        .unwrap();
}

fn command(
    command_id: &str,
    run_id: &str,
    expected_revision: u64,
    command: WorkRunCommand,
) -> WorkRunCommandRequest {
    WorkRunCommandRequest {
        command_id: CommandId::new(command_id).unwrap(),
        work_run_id: WorkRunId::new(run_id).unwrap(),
        expected_revision,
        command,
    }
}

fn dir(seed: &str) -> DirId {
    DirId::from_str(digest(seed).as_str()).unwrap()
}

fn digest(value: &str) -> ContentDigest {
    ContentDigest::sha256(value.as_bytes())
}

fn database_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-{label}-{}-{}.sqlite3",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
