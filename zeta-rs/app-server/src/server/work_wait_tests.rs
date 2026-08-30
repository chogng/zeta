use super::next_wait_resolution;
use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;
use zeta_file_access::DirId;
use zeta_file_access::EnvId;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;
use zeta_state::SqliteWorkRunStore;
use zeta_work_coordination::AuthorizationSnapshotRef;
use zeta_work_coordination::ExternalEffectsStatus;
use zeta_work_coordination::GitRepositoryCheckpoint;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::ManagedRootBinding;
use zeta_work_coordination::ResolveWaitOutcome;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::ValidationProfileRef;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkContractDraft;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkCoordinator;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRelationKind;
use zeta_work_coordination::WorkRelationStatus;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;
use zeta_work_coordination::WorkRunStore;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkStartMode;
use zeta_work_coordination::WorkWaitCondition;
use zeta_work_coordination::root_checkpoint_digest;

#[test]
fn sealed_target_resolves_the_exact_wait_without_polling_or_client_claims() {
    let directory = tempfile::tempdir().unwrap();
    let store: Arc<dyn WorkRunStore> =
        Arc::new(SqliteWorkRunStore::open(directory.path().join("state.sqlite3")).unwrap());
    let coordinator = WorkCoordinator::new(store);
    let run_id = WorkRunId::new("host-derived-wait").unwrap();
    apply(
        &coordinator,
        &run_id,
        0,
        "create-wait-run",
        WorkRunCommand::Create {
            objective: "wait for an exact sealed result".into(),
            acceptance_conditions: vec!["the source resumes once".into()],
            exclusions: Vec::new(),
            root_participant: participant("source-session", "source-thread"),
        },
    );
    apply(
        &coordinator,
        &run_id,
        1,
        "add-target-participant",
        WorkRunCommand::AddParticipant {
            participant: participant("target-session", "target-thread"),
        },
    );
    apply(
        &coordinator,
        &run_id,
        2,
        "create-source-contract",
        WorkRunCommand::CreateContract {
            contract: contract("source-contract", "source-thread"),
        },
    );
    apply(
        &coordinator,
        &run_id,
        3,
        "create-target-contract",
        WorkRunCommand::CreateContract {
            contract: contract("target-contract", "target-thread"),
        },
    );
    create_attempt(
        &coordinator,
        &run_id,
        4,
        "source-attempt",
        "source-contract",
        "source-thread",
    );
    create_attempt(
        &coordinator,
        &run_id,
        6,
        "target-attempt",
        "target-contract",
        "target-thread",
    );
    apply(
        &coordinator,
        &run_id,
        8,
        "begin-source",
        WorkRunCommand::BeginAttempt {
            attempt_id: WorkAttemptId::new("source-attempt").unwrap(),
            execution_id: WorkExecutionId::new("source-execution").unwrap(),
            mode: WorkStartMode::Write,
        },
    );
    apply(
        &coordinator,
        &run_id,
        9,
        "begin-target",
        WorkRunCommand::BeginAttempt {
            attempt_id: WorkAttemptId::new("target-attempt").unwrap(),
            execution_id: WorkExecutionId::new("target-execution").unwrap(),
            mode: WorkStartMode::Write,
        },
    );
    apply(
        &coordinator,
        &run_id,
        10,
        "wait-for-target",
        WorkRunCommand::CreateRelation {
            relation_id: WorkRelationId::new("source-waits-target").unwrap(),
            source_attempt_id: WorkAttemptId::new("source-attempt").unwrap(),
            target_attempt_id: WorkAttemptId::new("target-attempt").unwrap(),
            kind: WorkRelationKind::Wait {
                target_execution_id: WorkExecutionId::new("target-execution").unwrap(),
                condition: WorkWaitCondition::AttemptSealed,
            },
        },
    );
    let sealed = apply(
        &coordinator,
        &run_id,
        11,
        "seal-target",
        WorkRunCommand::SealAttempt {
            attempt_id: WorkAttemptId::new("target-attempt").unwrap(),
            result_digest: digest("target-result"),
            change_set_ids: Vec::new(),
            private_output_digest: digest("target-output"),
            external_effects_digest: digest("target-effects"),
            external_effects_status: ExternalEffectsStatus::None,
        },
    );

    let command = next_wait_resolution(&sealed).unwrap().unwrap();

    assert!(matches!(
        &command,
        WorkRunCommand::ResolveWait {
            outcome: ResolveWaitOutcome::Satisfied { evidence_digest },
            ..
        } if evidence_digest == &digest("target-result")
    ));
    let resumed = apply(
        &coordinator,
        &run_id,
        sealed.revision,
        "host-resolves-wait",
        command,
    );
    assert_eq!(
        resumed.attempts[&WorkAttemptId::new("source-attempt").unwrap()].execution_status,
        WorkAttemptExecutionStatus::Writing
    );
    assert_eq!(
        resumed.relations[&WorkRelationId::new("source-waits-target").unwrap()].status,
        WorkRelationStatus::Satisfied {
            evidence_digest: digest("target-result"),
        }
    );
}

fn create_attempt(
    coordinator: &WorkCoordinator,
    run_id: &WorkRunId,
    expected_revision: u64,
    attempt_id: &str,
    contract_id: &str,
    thread_id: &str,
) {
    let attempt_id = WorkAttemptId::new(attempt_id).unwrap();
    let created = apply(
        coordinator,
        run_id,
        expected_revision,
        &format!("create-{attempt_id}"),
        WorkRunCommand::CreateAttempt {
            attempt_id: attempt_id.clone(),
            contract: WorkContractRef {
                contract_id: WorkContractId::new(contract_id).unwrap(),
                revision: 1,
            },
            participant_thread_id: ThreadId::new(thread_id).unwrap(),
        },
    );
    let root = &created.attempts[&attempt_id].roots[0];
    apply(
        coordinator,
        run_id,
        expected_revision + 1,
        &format!("ready-{attempt_id}"),
        WorkRunCommand::RecordAttemptWorkspaceReady {
            attempt_id: attempt_id.clone(),
            roots: vec![ManagedRootBinding {
                source_dir_id: root.dir_id.clone(),
                managed_dir_id: dir(&format!("managed-{attempt_id}")),
                root_checkpoint_digest: root_checkpoint_digest(root).unwrap(),
                binding_manifest_digest: digest(&format!("binding-{attempt_id}")),
            }],
            private_output_dir_id: dir(&format!("output-{attempt_id}")),
        },
    );
}

fn contract(contract_id: &str, thread_id: &str) -> WorkContractDraft {
    let root_dir_id = dir("source-root");
    WorkContractDraft {
        contract_id: WorkContractId::new(contract_id).unwrap(),
        goal_revision: 1,
        topology_revision: 2,
        owner_thread_id: ThreadId::new(thread_id).unwrap(),
        objective: "execute bounded work".into(),
        acceptance_conditions: vec!["the bounded work is complete".into()],
        exclusions: Vec::new(),
        environment_id: EnvId::local(),
        roots: vec![RootCheckpoint {
            environment_id: EnvId::local(),
            dir_id: root_dir_id.clone(),
            state: RootState::Git {
                repositories: vec![GitRepositoryCheckpoint {
                    repository_id: "repository-a".into(),
                    relative_path: ".".into(),
                    target: GitRootTarget::Branch {
                        name: "main".into(),
                        expected_head: "head-a".into(),
                    },
                    baseline_tree: "tree-a".into(),
                }],
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
            content_digest: digest("profile"),
        },
    }
}

fn participant(session_id: &str, thread_id: &str) -> WorkParticipant {
    WorkParticipant {
        session_id: SessionId::new(session_id).unwrap(),
        thread_id: ThreadId::new(thread_id).unwrap(),
        relation: WorkParticipantRelation::Root,
    }
}

fn apply(
    coordinator: &WorkCoordinator,
    run_id: &WorkRunId,
    expected_revision: u64,
    command_id: &str,
    command: WorkRunCommand,
) -> zeta_work_coordination::WorkRun {
    coordinator
        .apply(WorkRunCommandRequest {
            command_id: CommandId::new(command_id).unwrap(),
            work_run_id: run_id.clone(),
            expected_revision,
            command,
        })
        .unwrap()
        .work_run
}

fn dir(seed: &str) -> DirId {
    DirId::from_str(ContentDigest::sha256(seed.as_bytes()).as_str()).unwrap()
}

fn digest(value: &str) -> ContentDigest {
    ContentDigest::sha256(value.as_bytes())
}
