use super::AuthorizationSnapshotRef;
use super::ExternalEffectsStatus;
use super::GitRepositoryCheckpoint;
use super::GitRootTarget;
use super::GitVerificationRepository;
use super::IntegrationFailureKind;
use super::IntegrationPreparedArtifact;
use super::ResolveWaitOutcome;
use super::RootCheckpoint;
use super::RootState;
use super::ValidationProfileRef;
use super::VerificationChangeSetInput;
use super::VerificationCheckEvidence;
use super::VerificationCheckOutcome;
use super::VerificationConclusion;
use super::VerificationRoot;
use super::VerificationRootState;
use super::WorkAttemptCoordinationStatus;
use super::WorkAttemptExecutionStatus;
use super::WorkAttemptIntegrationStatus;
use super::WorkAttemptVerificationStatus;
use super::WorkCommandDisposition;
use super::WorkContractDraft;
use super::WorkContractRef;
use super::WorkCoordinationError;
use super::WorkCoordinator;
use super::WorkIntegrationStatus;
use super::WorkParticipant;
use super::WorkParticipantRelation;
use super::WorkRelationKind;
use super::WorkRelationStatus;
use super::WorkRun;
use super::WorkRunCommand;
use super::WorkRunCommandRequest;
use super::WorkRunStore;
use super::WorkRunStoreError;
use super::WorkRunStoreOutcome;
use super::WorkScopeClaim;
use super::WorkSerializabilityEvidence;
use super::WorkSerializabilityStatus;
use super::WorkStartMode;
use super::WorkVerificationInput;
use super::WorkWaitCondition;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use zeta_environment::EnvId;
use zeta_file_access::DirId;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::DelegationId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;

#[derive(Default)]
struct MemoryStore {
    state: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    runs: BTreeMap<WorkRunId, WorkRun>,
    commands: BTreeMap<CommandId, super::WorkRunCommit>,
}

impl WorkRunStore for MemoryStore {
    fn list(&self) -> Result<Vec<WorkRun>, WorkRunStoreError> {
        Ok(self
            .state
            .lock()
            .expect("memory store lock")
            .runs
            .values()
            .cloned()
            .collect())
    }

    fn load(&self, work_run_id: &WorkRunId) -> Result<WorkRun, WorkRunStoreError> {
        self.state
            .lock()
            .expect("memory store lock")
            .runs
            .get(work_run_id)
            .cloned()
            .ok_or_else(|| WorkRunStoreError::NotFound(work_run_id.to_string()))
    }

    fn load_command(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<super::WorkRunCommit>, WorkRunStoreError> {
        Ok(self
            .state
            .lock()
            .expect("memory store lock")
            .commands
            .get(command_id)
            .cloned())
    }

    fn commit(
        &self,
        commit: &super::WorkRunCommit,
    ) -> Result<WorkRunStoreOutcome, WorkRunStoreError> {
        let mut state = self.state.lock().expect("memory store lock");
        if let Some(existing) = state.commands.get(&commit.request.command_id) {
            return if existing.request == commit.request {
                Ok(WorkRunStoreOutcome::Replayed(existing.result.clone()))
            } else {
                Err(WorkRunStoreError::CommandConflict)
            };
        }
        let actual = state
            .runs
            .get(&commit.request.work_run_id)
            .map_or(0, |run| run.revision);
        if actual != commit.request.expected_revision {
            return Err(WorkRunStoreError::RevisionConflict {
                expected: commit.request.expected_revision,
                actual,
            });
        }
        for writer in commit.result.active_writers() {
            if let Some((work_run_id, attempt)) = state
                .runs
                .iter()
                .filter(|(work_run_id, _)| *work_run_id != &commit.result.work_run_id)
                .find_map(|(work_run_id, run)| {
                    run.active_writers()
                        .find(|attempt| attempt.thread_id == writer.thread_id)
                        .map(|attempt| (work_run_id, attempt))
                })
            {
                return Err(WorkRunStoreError::ThreadBusy {
                    thread_id: writer.thread_id.clone(),
                    work_run_id: work_run_id.clone(),
                    attempt_id: attempt.attempt_id.clone(),
                });
            }
        }
        assert_eq!(commit.result.work_run_id, commit.request.work_run_id);
        assert_eq!(commit.result.revision, actual + 1);
        state
            .runs
            .insert(commit.result.work_run_id.clone(), commit.result.clone());
        state
            .commands
            .insert(commit.request.command_id.clone(), commit.clone());
        Ok(WorkRunStoreOutcome::Applied)
    }
}

#[test]
fn commands_are_revision_checked_and_exactly_replayed() {
    let coordinator = coordinator();
    let create = create_request("create-run", "run-1", participant("session-a", "thread-a"));
    let first = coordinator.apply(create.clone()).expect("create WorkRun");
    assert_eq!(first.disposition, WorkCommandDisposition::Committed);
    assert_eq!(first.work_run.revision, 1);

    let replay = coordinator.apply(create.clone()).expect("replay WorkRun");
    assert_eq!(replay.disposition, WorkCommandDisposition::Replayed);
    assert_eq!(replay.work_run, first.work_run);

    let mut conflicting = create;
    conflicting.command = WorkRunCommand::Create {
        objective: "different".into(),
        acceptance_conditions: vec!["accepted".into()],
        exclusions: Vec::new(),
        root_participant: participant("session-a", "thread-a"),
    };
    assert_eq!(
        coordinator.apply(conflicting),
        Err(WorkCoordinationError::CommandConflict)
    );

    let stale = request(
        "stale",
        "run-1",
        0,
        WorkRunCommand::ReviseGoal {
            objective: "new goal".into(),
            acceptance_conditions: vec!["accepted".into()],
            exclusions: Vec::new(),
        },
    );
    assert_eq!(
        coordinator.apply(stale),
        Err(WorkCoordinationError::RevisionConflict {
            expected: 0,
            actual: 1,
        })
    );
}

#[test]
fn topology_keeps_agent_tree_and_independent_sessions_distinct() {
    let coordinator = coordinator();
    let run_id = WorkRunId::new("topology").unwrap();
    coordinator
        .apply(create_request(
            "create-topology",
            run_id.as_str(),
            participant("session-a", "thread-a"),
        ))
        .unwrap();
    let delegated = WorkParticipant {
        session_id: SessionId::new("session-a").unwrap(),
        thread_id: ThreadId::new("thread-a-child").unwrap(),
        relation: WorkParticipantRelation::Delegated {
            parent_thread_id: ThreadId::new("thread-a").unwrap(),
            delegation_id: DelegationId::new("delegation-a").unwrap(),
        },
    };
    coordinator
        .apply(request(
            "add-child",
            run_id.as_str(),
            1,
            WorkRunCommand::AddParticipant {
                participant: delegated,
            },
        ))
        .unwrap();
    let two_sessions = coordinator
        .apply(request(
            "add-peer",
            run_id.as_str(),
            2,
            WorkRunCommand::AddParticipant {
                participant: participant("session-b", "thread-b"),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(two_sessions.session_count(), 2);
    assert_eq!(two_sessions.topology_revision, 3);

    let invalid = WorkParticipant {
        session_id: SessionId::new("session-b").unwrap(),
        thread_id: ThreadId::new("cross-session-child").unwrap(),
        relation: WorkParticipantRelation::Delegated {
            parent_thread_id: ThreadId::new("thread-a").unwrap(),
            delegation_id: DelegationId::new("delegation-cross").unwrap(),
        },
    };
    assert!(matches!(
        coordinator.apply(request(
            "invalid-child",
            run_id.as_str(),
            3,
            WorkRunCommand::AddParticipant {
                participant: invalid,
            },
        )),
        Err(WorkCoordinationError::InvalidInput(_))
    ));
}

#[test]
fn contract_revision_stales_an_active_attempt() {
    let coordinator = coordinator();
    create_run_contract_attempt(&coordinator, "stale-contract", "owner", "attempt-a");
    coordinator
        .apply(request(
            "begin-attempt",
            "stale-contract",
            4,
            WorkRunCommand::BeginAttempt {
                attempt_id: WorkAttemptId::new("attempt-a").unwrap(),
                execution_id: WorkExecutionId::new("execution-a").unwrap(),
                mode: WorkStartMode::Write,
            },
        ))
        .unwrap();
    let revised = coordinator
        .apply(request(
            "revise-contract",
            "stale-contract",
            5,
            WorkRunCommand::ReviseContract {
                expected_contract_revision: 1,
                contract: contract("contract-owner", "owner"),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        revised.attempts[&WorkAttemptId::new("attempt-a").unwrap()].coordination_status,
        WorkAttemptCoordinationStatus::Stale
    );
    assert_eq!(
        revised.contracts[&WorkContractId::new("contract-owner").unwrap()].len(),
        2
    );
}

#[test]
fn topology_revision_stops_active_work_and_rejects_old_contracts() {
    let coordinator = coordinator();
    create_run_contract_attempt(&coordinator, "stale-topology", "owner", "attempt-a");
    coordinator
        .apply(request(
            "begin-topology-attempt",
            "stale-topology",
            4,
            WorkRunCommand::BeginAttempt {
                attempt_id: WorkAttemptId::new("attempt-a").unwrap(),
                execution_id: WorkExecutionId::new("execution-a").unwrap(),
                mode: WorkStartMode::Write,
            },
        ))
        .unwrap();
    let changed = coordinator
        .apply(request(
            "add-topology-participant",
            "stale-topology",
            5,
            WorkRunCommand::AddParticipant {
                participant: participant("session-peer", "peer"),
            },
        ))
        .unwrap()
        .work_run;
    let attempt = &changed.attempts[&WorkAttemptId::new("attempt-a").unwrap()];
    assert_eq!(
        attempt.execution_status,
        WorkAttemptExecutionStatus::Interrupted
    );
    assert_eq!(
        attempt.coordination_status,
        WorkAttemptCoordinationStatus::Stale
    );
    assert_eq!(
        attempt.verification_status,
        WorkAttemptVerificationStatus::Stale
    );

    assert!(matches!(
        coordinator.apply(request(
            "old-topology-contract",
            "stale-topology",
            6,
            WorkRunCommand::CreateContract {
                contract: contract("old-topology", "owner"),
            },
        )),
        Err(WorkCoordinationError::InvalidInput(message))
            if message.contains("topology")
    ));
}

#[test]
fn wait_resumes_only_for_the_exact_attempt_and_execution() {
    let coordinator = coordinator();
    let run_id = "wait-run";
    coordinator
        .apply(create_request(
            "create-wait",
            run_id,
            participant("session-a", "thread-a"),
        ))
        .unwrap();
    coordinator
        .apply(request(
            "add-wait-peer",
            run_id,
            1,
            WorkRunCommand::AddParticipant {
                participant: participant("session-b", "thread-b"),
            },
        ))
        .unwrap();
    coordinator
        .apply(request(
            "contract-a",
            run_id,
            2,
            WorkRunCommand::CreateContract {
                contract: contract_at_topology("contract-a", "thread-a", 2),
            },
        ))
        .unwrap();
    coordinator
        .apply(request(
            "contract-b",
            run_id,
            3,
            WorkRunCommand::CreateContract {
                contract: contract_at_topology("contract-b", "thread-b", 2),
            },
        ))
        .unwrap();
    create_attempt(
        &coordinator,
        run_id,
        4,
        "attempt-a",
        "contract-a",
        "thread-a",
    );
    create_attempt(
        &coordinator,
        run_id,
        6,
        "attempt-b",
        "contract-b",
        "thread-b",
    );
    begin_attempt(&coordinator, run_id, 8, "attempt-a", "execution-a");
    begin_attempt(&coordinator, run_id, 9, "attempt-b", "execution-b");
    let waiting = coordinator
        .apply(request(
            "wait-for-b",
            run_id,
            10,
            WorkRunCommand::CreateRelation {
                relation_id: WorkRelationId::new("wait-a-b").unwrap(),
                source_attempt_id: WorkAttemptId::new("attempt-a").unwrap(),
                target_attempt_id: WorkAttemptId::new("attempt-b").unwrap(),
                kind: WorkRelationKind::Wait {
                    target_execution_id: WorkExecutionId::new("execution-b").unwrap(),
                    condition: WorkWaitCondition::AttemptSealed,
                },
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        waiting.attempts[&WorkAttemptId::new("attempt-a").unwrap()].execution_status,
        WorkAttemptExecutionStatus::Waiting
    );

    assert!(matches!(
        coordinator.apply(request(
            "wrong-wakeup",
            run_id,
            11,
            WorkRunCommand::ResolveWait {
                relation_id: WorkRelationId::new("wait-a-b").unwrap(),
                target_attempt_id: WorkAttemptId::new("attempt-b").unwrap(),
                target_execution_id: WorkExecutionId::new("wrong-generation").unwrap(),
                outcome: ResolveWaitOutcome::Satisfied {
                    evidence_digest: digest("result-b"),
                },
            },
        )),
        Err(WorkCoordinationError::InvalidInput(_))
    ));
    coordinator
        .apply(request(
            "seal-b",
            run_id,
            11,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt-b").unwrap(),
                result_digest: digest("result-b"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("output-b"),
                external_effects_digest: digest("no-external-effects"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();
    let resumed = coordinator
        .apply(request(
            "wake-a",
            run_id,
            12,
            WorkRunCommand::ResolveWait {
                relation_id: WorkRelationId::new("wait-a-b").unwrap(),
                target_attempt_id: WorkAttemptId::new("attempt-b").unwrap(),
                target_execution_id: WorkExecutionId::new("execution-b").unwrap(),
                outcome: ResolveWaitOutcome::Satisfied {
                    evidence_digest: digest("result-b"),
                },
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        resumed.relations[&WorkRelationId::new("wait-a-b").unwrap()].status,
        WorkRelationStatus::Satisfied {
            evidence_digest: digest("result-b"),
        }
    );
    assert_eq!(
        resumed.attempts[&WorkAttemptId::new("attempt-a").unwrap()].execution_status,
        WorkAttemptExecutionStatus::Writing
    );
}

#[test]
fn dependency_cycles_are_rejected_before_the_second_attempt_waits() {
    let coordinator = coordinator();
    let run_id = "cycle-run";
    coordinator
        .apply(create_request(
            "create-cycle",
            run_id,
            participant("session-a", "thread-a"),
        ))
        .unwrap();
    coordinator
        .apply(request(
            "add-cycle-peer",
            run_id,
            1,
            WorkRunCommand::AddParticipant {
                participant: participant("session-b", "thread-b"),
            },
        ))
        .unwrap();
    for (revision, suffix) in [(2, "a"), (3, "b")] {
        coordinator
            .apply(request(
                &format!("cycle-contract-{suffix}"),
                run_id,
                revision,
                WorkRunCommand::CreateContract {
                    contract: contract_at_topology(
                        &format!("cycle-contract-{suffix}"),
                        &format!("thread-{suffix}"),
                        2,
                    ),
                },
            ))
            .unwrap();
    }
    create_attempt(
        &coordinator,
        run_id,
        4,
        "cycle-attempt-a",
        "cycle-contract-a",
        "thread-a",
    );
    create_attempt(
        &coordinator,
        run_id,
        6,
        "cycle-attempt-b",
        "cycle-contract-b",
        "thread-b",
    );
    begin_attempt(
        &coordinator,
        run_id,
        8,
        "cycle-attempt-a",
        "cycle-execution-a",
    );
    begin_attempt(
        &coordinator,
        run_id,
        9,
        "cycle-attempt-b",
        "cycle-execution-b",
    );
    coordinator
        .apply(request(
            "cycle-a-waits-b",
            run_id,
            10,
            WorkRunCommand::CreateRelation {
                relation_id: WorkRelationId::new("cycle-a-b").unwrap(),
                source_attempt_id: WorkAttemptId::new("cycle-attempt-a").unwrap(),
                target_attempt_id: WorkAttemptId::new("cycle-attempt-b").unwrap(),
                kind: WorkRelationKind::Wait {
                    target_execution_id: WorkExecutionId::new("cycle-execution-b").unwrap(),
                    condition: WorkWaitCondition::ExecutionFinished,
                },
            },
        ))
        .unwrap();
    assert!(matches!(
        coordinator.apply(request(
            "cycle-b-waits-a",
            run_id,
            11,
            WorkRunCommand::CreateRelation {
                relation_id: WorkRelationId::new("cycle-b-a").unwrap(),
                source_attempt_id: WorkAttemptId::new("cycle-attempt-b").unwrap(),
                target_attempt_id: WorkAttemptId::new("cycle-attempt-a").unwrap(),
                kind: WorkRelationKind::Wait {
                    target_execution_id: WorkExecutionId::new("cycle-execution-a").unwrap(),
                    condition: WorkWaitCondition::ExecutionFinished,
                },
            },
        )),
        Err(WorkCoordinationError::InvalidInput(message))
            if message.contains("cycle")
    ));
}

#[test]
fn attempt_requires_an_exact_ready_managed_root_set() {
    let coordinator = coordinator();
    let run_id = "workspace-run";
    coordinator
        .apply(create_request(
            "create-workspace-run",
            run_id,
            participant("workspace-session", "workspace-thread"),
        ))
        .unwrap();
    let mut draft = contract("workspace-contract", "workspace-thread");
    draft.roots.push(RootCheckpoint {
        environment_id: EnvId::local(),
        dir_id: test_dir("second-source"),
        state: RootState::Git {
            repositories: vec![GitRepositoryCheckpoint {
                repository_id: "repository-b".into(),
                relative_path: ".".into(),
                target: GitRootTarget::Detached {
                    object_id: "head-b".into(),
                },
                baseline_tree: "tree-b".into(),
            }],
        },
        control_resources: Vec::new(),
    });
    coordinator
        .apply(request(
            "create-workspace-contract",
            run_id,
            1,
            WorkRunCommand::CreateContract { contract: draft },
        ))
        .unwrap();
    create_planned_attempt(
        &coordinator,
        run_id,
        2,
        "workspace-attempt",
        "workspace-contract",
        "workspace-thread",
    );

    assert!(matches!(
        coordinator.apply(request(
            "begin-without-workspace",
            run_id,
            3,
            WorkRunCommand::BeginAttempt {
                attempt_id: WorkAttemptId::new("workspace-attempt").unwrap(),
                execution_id: WorkExecutionId::new("workspace-execution").unwrap(),
                mode: WorkStartMode::Write,
            },
        )),
        Err(WorkCoordinationError::InvalidTransition(message))
            if message.contains("managed root set")
    ));

    let run = coordinator.read(&WorkRunId::new(run_id).unwrap()).unwrap();
    let attempt_id = WorkAttemptId::new("workspace-attempt").unwrap();
    let mut incomplete = managed_roots(&run, &attempt_id);
    incomplete.pop();
    assert!(matches!(
        coordinator.apply(request(
            "incomplete-workspace",
            run_id,
            3,
            WorkRunCommand::RecordAttemptWorkspaceReady {
                attempt_id: attempt_id.clone(),
                roots: incomplete,
                private_output_dir_id: test_dir("workspace-output"),
            },
        )),
        Err(WorkCoordinationError::InvalidInput(message))
            if message.contains("every contract root")
    ));

    mark_workspace_ready(&coordinator, run_id, 3, &attempt_id);
    let begun = coordinator
        .apply(request(
            "begin-ready-workspace",
            run_id,
            4,
            WorkRunCommand::BeginAttempt {
                attempt_id: attempt_id.clone(),
                execution_id: WorkExecutionId::new("workspace-execution").unwrap(),
                mode: WorkStartMode::Write,
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        begun.attempts[&attempt_id].execution_status,
        WorkAttemptExecutionStatus::Writing
    );
}

#[test]
fn one_thread_writer_is_enforced_across_work_runs() {
    let store = Arc::new(MemoryStore::default());
    let coordinator = WorkCoordinator::new(store);
    create_run_contract_attempt(&coordinator, "writer-run-a", "shared-thread", "attempt-a");
    create_run_contract_attempt(&coordinator, "writer-run-b", "shared-thread", "attempt-b");
    begin_attempt(&coordinator, "writer-run-a", 4, "attempt-a", "execution-a");
    let second_begin = request(
        "begin-attempt-b",
        "writer-run-b",
        4,
        WorkRunCommand::BeginAttempt {
            attempt_id: WorkAttemptId::new("attempt-b").unwrap(),
            execution_id: WorkExecutionId::new("execution-b").unwrap(),
            mode: WorkStartMode::Write,
        },
    );
    assert!(matches!(
        coordinator.apply(second_begin.clone()),
        Err(WorkCoordinationError::ThreadBusy { thread_id, .. })
            if thread_id == ThreadId::new("shared-thread").unwrap()
    ));
    coordinator
        .apply(request(
            "seal-attempt-a",
            "writer-run-a",
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
    assert_eq!(
        coordinator.apply(second_begin).unwrap().work_run.attempts
            [&WorkAttemptId::new("attempt-b").unwrap()]
            .execution_status,
        WorkAttemptExecutionStatus::Writing
    );
}

#[test]
fn verification_binds_exact_inputs_and_stale_evidence_cannot_be_reused() {
    let coordinator = coordinator();
    create_run_contract_attempt(&coordinator, "verification-run", "worker", "attempt");
    begin_attempt(&coordinator, "verification-run", 4, "attempt", "execution");
    coordinator
        .apply(request(
            "seal-verification-attempt",
            "verification-run",
            5,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt").unwrap(),
                result_digest: digest("result"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("output"),
                external_effects_digest: digest("effects"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();
    let run_id = WorkRunId::new("verification-run").unwrap();
    let run = coordinator.read(&run_id).unwrap();
    let first_input = verification_input(&run, "target-tree-a", "final-tree-a");
    let first_key = super::verification_key(&run_id, &first_input).unwrap();
    let verifying = coordinator
        .apply(request(
            "begin-verification",
            "verification-run",
            6,
            WorkRunCommand::BeginVerification { input: first_input },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        verifying.attempts[&WorkAttemptId::new("attempt").unwrap()].verification_status,
        WorkAttemptVerificationStatus::Verifying
    );
    let failed_check = VerificationCheckEvidence {
        check_id: "compile".into(),
        command_digest: digest("compile-command"),
        output_digest: digest("compile-failure"),
        outcome: VerificationCheckOutcome::Failed,
    };
    assert!(matches!(
        coordinator.apply(request(
            "false-verification-pass",
            "verification-run",
            7,
            WorkRunCommand::FinishVerification {
                verification_key: first_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![failed_check],
                reason: "claimed pass".into(),
            },
        )),
        Err(WorkCoordinationError::InvalidInput(message))
            if message.contains("disagrees")
    ));
    let verified = coordinator
        .apply(request(
            "finish-verification",
            "verification-run",
            7,
            WorkRunCommand::FinishVerification {
                verification_key: first_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![VerificationCheckEvidence {
                    check_id: "compile".into(),
                    command_digest: digest("compile-command"),
                    output_digest: digest("compile-success"),
                    outcome: VerificationCheckOutcome::Passed,
                }],
                reason: "trusted compile check passed".into(),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        verified.attempts[&WorkAttemptId::new("attempt").unwrap()].verification_status,
        WorkAttemptVerificationStatus::Verified
    );
    let stale = coordinator
        .apply(request(
            "target-moved",
            "verification-run",
            8,
            WorkRunCommand::MarkVerificationStale {
                verification_key: first_key.clone(),
                reason: "target HEAD moved".into(),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        stale.attempts[&WorkAttemptId::new("attempt").unwrap()].verification_status,
        WorkAttemptVerificationStatus::Stale
    );
    assert!(matches!(
        coordinator.apply(request(
            "reuse-old-input",
            "verification-run",
            9,
            WorkRunCommand::BeginVerification {
                input: verification_input(&stale, "target-tree-a", "final-tree-a"),
            },
        )),
        Err(WorkCoordinationError::AlreadyExists(identity)) if identity == first_key.to_string()
    ));
    let second_input = verification_input(&stale, "target-tree-b", "final-tree-b");
    let second_key = super::verification_key(&run_id, &second_input).unwrap();
    let reverifying = coordinator
        .apply(request(
            "verify-new-target",
            "verification-run",
            9,
            WorkRunCommand::BeginVerification {
                input: second_input,
            },
        ))
        .unwrap()
        .work_run;
    assert_ne!(first_key, second_key);
    assert_eq!(
        reverifying.attempts[&WorkAttemptId::new("attempt").unwrap()].verification_status,
        WorkAttemptVerificationStatus::Verifying
    );
}

#[test]
fn a_new_relation_invalidates_verified_coordination_evidence() {
    let coordinator = coordinator();
    let run_id = "relation-stales-verification";
    create_run_contract_attempt(&coordinator, run_id, "worker", "attempt");
    coordinator
        .apply(request(
            "create-related-contract",
            run_id,
            4,
            WorkRunCommand::CreateContract {
                contract: contract("related-contract", "worker"),
            },
        ))
        .unwrap();
    create_attempt(
        &coordinator,
        run_id,
        5,
        "related-attempt",
        "related-contract",
        "worker",
    );
    begin_attempt(&coordinator, run_id, 7, "attempt", "execution");
    coordinator
        .apply(request(
            "seal-primary-attempt",
            run_id,
            8,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt").unwrap(),
                result_digest: digest("primary-result"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("primary-output"),
                external_effects_digest: digest("primary-effects"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();
    begin_attempt(
        &coordinator,
        run_id,
        9,
        "related-attempt",
        "related-execution",
    );
    coordinator
        .apply(request(
            "seal-related-attempt",
            run_id,
            10,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("related-attempt").unwrap(),
                result_digest: digest("related-result"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("related-output"),
                external_effects_digest: digest("related-effects"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();

    let before_relation = coordinator.read(&WorkRunId::new(run_id).unwrap()).unwrap();
    let input = verification_input(&before_relation, "target-tree", "final-tree");
    let old_coordination_digest = input.coordination_digest.clone();
    let verification_key = super::verification_key(&before_relation.work_run_id, &input).unwrap();
    coordinator
        .apply(request(
            "begin-relation-verification",
            run_id,
            11,
            WorkRunCommand::BeginVerification { input },
        ))
        .unwrap();
    coordinator
        .apply(request(
            "finish-relation-verification",
            run_id,
            12,
            WorkRunCommand::FinishVerification {
                verification_key: verification_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![VerificationCheckEvidence {
                    check_id: "compile".into(),
                    command_digest: digest("compile-command"),
                    output_digest: digest("compile-success"),
                    outcome: VerificationCheckOutcome::Passed,
                }],
                reason: "the exact pre-relation state passed".into(),
            },
        ))
        .unwrap();

    let changed = coordinator
        .apply(request(
            "record-observation-relation",
            run_id,
            13,
            WorkRunCommand::CreateRelation {
                relation_id: WorkRelationId::new("primary-observes-related").unwrap(),
                source_attempt_id: WorkAttemptId::new("attempt").unwrap(),
                target_attempt_id: WorkAttemptId::new("related-attempt").unwrap(),
                kind: WorkRelationKind::Observation,
            },
        ))
        .unwrap()
        .work_run;

    assert_eq!(
        changed.verifications[&verification_key].status,
        super::WorkVerificationStatus::Stale
    );
    assert_eq!(
        changed.attempts[&WorkAttemptId::new("attempt").unwrap()].verification_status,
        WorkAttemptVerificationStatus::Stale
    );
    assert_ne!(
        verification_input(&changed, "target-tree", "final-tree").coordination_digest,
        old_coordination_digest
    );
}

#[test]
fn unknown_external_effects_can_only_finish_indeterminate() {
    let coordinator = coordinator();
    create_run_contract_attempt(&coordinator, "unknown-effect-run", "worker", "attempt");
    begin_attempt(
        &coordinator,
        "unknown-effect-run",
        4,
        "attempt",
        "execution",
    );
    coordinator
        .apply(request(
            "seal-unknown-effect-attempt",
            "unknown-effect-run",
            5,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt").unwrap(),
                result_digest: digest("result"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("output"),
                external_effects_digest: digest("unknown-effect"),
                external_effects_status: ExternalEffectsStatus::Unknown,
            },
        ))
        .unwrap();
    let run_id = WorkRunId::new("unknown-effect-run").unwrap();
    let run = coordinator.read(&run_id).unwrap();
    let input = verification_input(&run, "target-tree", "final-tree");
    let verification_key = super::verification_key(&run_id, &input).unwrap();
    coordinator
        .apply(request(
            "begin-unknown-effect-verification",
            "unknown-effect-run",
            6,
            WorkRunCommand::BeginVerification { input },
        ))
        .unwrap();
    let passed_check = VerificationCheckEvidence {
        check_id: "acceptance".into(),
        command_digest: digest("command"),
        output_digest: digest("output"),
        outcome: VerificationCheckOutcome::Passed,
    };
    assert!(matches!(
        coordinator.apply(request(
            "reject-unknown-effect-verification",
            "unknown-effect-run",
            7,
            WorkRunCommand::FinishVerification {
                verification_key: verification_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![passed_check],
                reason: "the code checks passed".into(),
            },
        )),
        Err(WorkCoordinationError::InvalidInput(message))
            if message.contains("unknown external effects")
    ));
    let indeterminate = coordinator
        .apply(request(
            "record-unknown-effect-verification",
            "unknown-effect-run",
            7,
            WorkRunCommand::FinishVerification {
                verification_key,
                conclusion: VerificationConclusion::Indeterminate,
                checks: vec![VerificationCheckEvidence {
                    check_id: "external-effects".into(),
                    command_digest: digest("effect-reconciliation"),
                    output_digest: digest("missing-receipt"),
                    outcome: VerificationCheckOutcome::Indeterminate,
                }],
                reason: "an executed external effect has no verifiable receipt".into(),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        indeterminate.attempts[&WorkAttemptId::new("attempt").unwrap()].verification_status,
        WorkAttemptVerificationStatus::Indeterminate
    );
}

#[test]
fn indeterminate_serializability_can_only_finish_indeterminate() {
    let coordinator = coordinator();
    create_run_contract_attempt(&coordinator, "indeterminate-order-run", "worker", "attempt");
    begin_attempt(
        &coordinator,
        "indeterminate-order-run",
        4,
        "attempt",
        "execution",
    );
    coordinator
        .apply(request(
            "seal-indeterminate-order-attempt",
            "indeterminate-order-run",
            5,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt").unwrap(),
                result_digest: digest("result"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("output"),
                external_effects_digest: digest("effects"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();
    let run_id = WorkRunId::new("indeterminate-order-run").unwrap();
    let run = coordinator.read(&run_id).unwrap();
    let mut input = verification_input(&run, "target-tree", "final-tree");
    input.serializability = WorkSerializabilityEvidence {
        status: WorkSerializabilityStatus::Indeterminate,
        evidence_digest: digest("overlapping-write-evidence"),
        reason: "two WorkAttempts wrote the same file without an exact dependency".into(),
    };
    let verification_key = super::verification_key(&run_id, &input).unwrap();
    coordinator
        .apply(request(
            "begin-indeterminate-order-verification",
            "indeterminate-order-run",
            6,
            WorkRunCommand::BeginVerification { input },
        ))
        .unwrap();
    assert!(matches!(
        coordinator.apply(request(
            "reject-indeterminate-order-verification",
            "indeterminate-order-run",
            7,
            WorkRunCommand::FinishVerification {
                verification_key: verification_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![VerificationCheckEvidence {
                    check_id: "acceptance".into(),
                    command_digest: digest("acceptance-command"),
                    output_digest: digest("acceptance-output"),
                    outcome: VerificationCheckOutcome::Passed,
                }],
                reason: "the replay checks passed".into(),
            },
        )),
        Err(WorkCoordinationError::InvalidInput(message))
            if message.contains("serializability")
    ));
    let indeterminate = coordinator
        .apply(request(
            "record-indeterminate-order-verification",
            "indeterminate-order-run",
            7,
            WorkRunCommand::FinishVerification {
                verification_key,
                conclusion: VerificationConclusion::Indeterminate,
                checks: vec![VerificationCheckEvidence {
                    check_id: "serializability".into(),
                    command_digest: digest("actual-effect-order"),
                    output_digest: digest("overlapping-write-evidence"),
                    outcome: VerificationCheckOutcome::Indeterminate,
                }],
                reason: "the candidate has no proven serial execution order".into(),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        indeterminate.attempts[&WorkAttemptId::new("attempt").unwrap()].verification_status,
        WorkAttemptVerificationStatus::Indeterminate
    );
}

#[test]
fn integration_publishes_only_prepared_roots_in_order() {
    let coordinator = coordinator();
    let run_id = "integration-run";
    create_run_contract_attempt(&coordinator, run_id, "worker", "attempt");
    begin_attempt(&coordinator, run_id, 4, "attempt", "execution");
    coordinator
        .apply(request(
            "seal-integration-attempt",
            run_id,
            5,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt").unwrap(),
                result_digest: digest("integration-result"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("integration-output"),
                external_effects_digest: digest("integration-effects"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();
    let work_run_id = WorkRunId::new(run_id).unwrap();
    let run = coordinator.read(&work_run_id).unwrap();
    let input = verification_input(&run, "target-tree", "final-tree");
    let verification_key = super::verification_key(&work_run_id, &input).unwrap();
    coordinator
        .apply(request(
            "begin-integration-verification",
            run_id,
            6,
            WorkRunCommand::BeginVerification { input },
        ))
        .unwrap();
    coordinator
        .apply(request(
            "finish-integration-verification",
            run_id,
            7,
            WorkRunCommand::FinishVerification {
                verification_key: verification_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![VerificationCheckEvidence {
                    check_id: "acceptance".into(),
                    command_digest: digest("acceptance-command"),
                    output_digest: digest("acceptance-output"),
                    outcome: VerificationCheckOutcome::Passed,
                }],
                reason: "trusted acceptance check passed".into(),
            },
        ))
        .unwrap();
    let queued = coordinator
        .apply(request(
            "queue-integration",
            run_id,
            8,
            WorkRunCommand::QueueIntegration {
                verification_key: verification_key.clone(),
            },
        ))
        .unwrap()
        .work_run;
    let integration_key = super::integration_key(&work_run_id, &verification_key).unwrap();
    let root_id = queued.integrations[&integration_key].roots[0]
        .root_id
        .clone();
    assert_eq!(
        queued.attempts[&WorkAttemptId::new("attempt").unwrap()].integration_status,
        WorkAttemptIntegrationStatus::Queued
    );
    assert!(matches!(
        coordinator.apply(request(
            "begin-before-preparation",
            run_id,
            9,
            WorkRunCommand::BeginIntegration {
                integration_key: integration_key.clone(),
                generation: 1,
            },
        )),
        Err(WorkCoordinationError::InvalidTransition(message))
            if message.contains("prepared")
    ));
    coordinator
        .apply(request(
            "prepare-integration-root",
            run_id,
            9,
            WorkRunCommand::RecordIntegrationRootPrepared {
                integration_key: integration_key.clone(),
                generation: 1,
                root_id: root_id.clone(),
                artifact: IntegrationPreparedArtifact::GitCommit {
                    object_id: "commit-a".into(),
                },
            },
        ))
        .unwrap();
    coordinator
        .apply(request(
            "begin-integration-publication",
            run_id,
            10,
            WorkRunCommand::BeginIntegration {
                integration_key: integration_key.clone(),
                generation: 1,
            },
        ))
        .unwrap();
    assert!(matches!(
        coordinator.apply(request(
            "late-generation-publication",
            run_id,
            11,
            WorkRunCommand::RecordIntegrationRootPublished {
                integration_key: integration_key.clone(),
                generation: 2,
                root_id: root_id.clone(),
                receipt_digest: digest("receipt-a"),
            },
        )),
        Err(WorkCoordinationError::InvalidTransition(message))
            if message.contains("generation")
    ));
    let integrated = coordinator
        .apply(request(
            "publish-integration-root",
            run_id,
            11,
            WorkRunCommand::RecordIntegrationRootPublished {
                integration_key: integration_key.clone(),
                generation: 1,
                root_id,
                receipt_digest: digest("receipt-a"),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        integrated.integrations[&integration_key].status,
        WorkIntegrationStatus::Integrated
    );
    assert!(
        integrated.integrations[&integration_key]
            .evidence_digest
            .is_some()
    );
    assert_eq!(
        integrated.attempts[&WorkAttemptId::new("attempt").unwrap()].integration_status,
        WorkAttemptIntegrationStatus::Integrated
    );
}

#[test]
fn multi_repository_failure_is_partial_and_target_movement_stales_verification() {
    let coordinator = coordinator();
    let run_id = "partial-integration";
    coordinator
        .apply(create_request(
            "create-partial-integration",
            run_id,
            participant("session-owner", "worker"),
        ))
        .unwrap();
    let mut draft = contract("contract-owner", "worker");
    let RootState::Git { repositories } = &mut draft.roots[0].state else {
        panic!("test contract uses a Git root")
    };
    repositories.push(GitRepositoryCheckpoint {
        repository_id: "repository-b".into(),
        relative_path: "nested".into(),
        target: GitRootTarget::Branch {
            name: "main".into(),
            expected_head: "head-b".into(),
        },
        baseline_tree: "tree-b".into(),
    });
    coordinator
        .apply(request(
            "create-partial-contract",
            run_id,
            1,
            WorkRunCommand::CreateContract { contract: draft },
        ))
        .unwrap();
    create_attempt(
        &coordinator,
        run_id,
        2,
        "attempt",
        "contract-owner",
        "worker",
    );
    begin_attempt(&coordinator, run_id, 4, "attempt", "execution");
    coordinator
        .apply(request(
            "seal-partial-attempt",
            run_id,
            5,
            WorkRunCommand::SealAttempt {
                attempt_id: WorkAttemptId::new("attempt").unwrap(),
                result_digest: digest("partial-result"),
                change_set_ids: Vec::new(),
                private_output_digest: digest("partial-output"),
                external_effects_digest: digest("partial-effects"),
                external_effects_status: ExternalEffectsStatus::None,
            },
        ))
        .unwrap();
    let work_run_id = WorkRunId::new(run_id).unwrap();
    let run = coordinator.read(&work_run_id).unwrap();
    let input = verification_input(&run, "target-tree", "final-tree");
    let verification_key = super::verification_key(&work_run_id, &input).unwrap();
    coordinator
        .apply(request(
            "begin-partial-verification",
            run_id,
            6,
            WorkRunCommand::BeginVerification { input },
        ))
        .unwrap();
    coordinator
        .apply(request(
            "finish-partial-verification",
            run_id,
            7,
            WorkRunCommand::FinishVerification {
                verification_key: verification_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![VerificationCheckEvidence {
                    check_id: "acceptance".into(),
                    command_digest: digest("partial-command"),
                    output_digest: digest("partial-output"),
                    outcome: VerificationCheckOutcome::Passed,
                }],
                reason: "all roots passed together".into(),
            },
        ))
        .unwrap();
    let queued = coordinator
        .apply(request(
            "queue-partial-integration",
            run_id,
            8,
            WorkRunCommand::QueueIntegration {
                verification_key: verification_key.clone(),
            },
        ))
        .unwrap()
        .work_run;
    let integration_key = super::integration_key(&work_run_id, &verification_key).unwrap();
    let roots = queued.integrations[&integration_key].roots.clone();
    assert_eq!(roots.len(), 2);
    for (offset, root) in roots.iter().enumerate() {
        coordinator
            .apply(request(
                &format!("prepare-partial-root-{offset}"),
                run_id,
                9 + u64::try_from(offset).unwrap(),
                WorkRunCommand::RecordIntegrationRootPrepared {
                    integration_key: integration_key.clone(),
                    generation: 1,
                    root_id: root.root_id.clone(),
                    artifact: IntegrationPreparedArtifact::GitCommit {
                        object_id: format!("commit-{offset}"),
                    },
                },
            ))
            .unwrap();
    }
    coordinator
        .apply(request(
            "begin-partial-publication",
            run_id,
            11,
            WorkRunCommand::BeginIntegration {
                integration_key: integration_key.clone(),
                generation: 1,
            },
        ))
        .unwrap();
    assert!(matches!(
        coordinator.apply(request(
            "publish-second-root-first",
            run_id,
            12,
            WorkRunCommand::RecordIntegrationRootPublished {
                integration_key: integration_key.clone(),
                generation: 1,
                root_id: roots[1].root_id.clone(),
                receipt_digest: digest("receipt-b"),
            },
        )),
        Err(WorkCoordinationError::InvalidTransition(message))
            if message.contains("recorded order")
    ));
    coordinator
        .apply(request(
            "publish-first-partial-root",
            run_id,
            12,
            WorkRunCommand::RecordIntegrationRootPublished {
                integration_key: integration_key.clone(),
                generation: 1,
                root_id: roots[0].root_id.clone(),
                receipt_digest: digest("receipt-a"),
            },
        ))
        .unwrap();
    let partial = coordinator
        .apply(request(
            "second-target-moved",
            run_id,
            13,
            WorkRunCommand::FailIntegration {
                integration_key: integration_key.clone(),
                generation: 1,
                kind: IntegrationFailureKind::TargetMoved,
                reason: "repository-b HEAD moved".into(),
            },
        ))
        .unwrap()
        .work_run;
    assert_eq!(
        partial.integrations[&integration_key].status,
        WorkIntegrationStatus::Partial
    );
    assert_eq!(
        partial.attempts[&WorkAttemptId::new("attempt").unwrap()].integration_status,
        WorkAttemptIntegrationStatus::Partial
    );
    assert_eq!(
        partial.verifications[&verification_key].status,
        super::WorkVerificationStatus::Stale
    );
    assert!(matches!(
        coordinator.apply(request(
            "resume-stale-partial",
            run_id,
            14,
            WorkRunCommand::ResumeIntegration {
                integration_key,
                generation: 1,
            },
        )),
        Err(WorkCoordinationError::InvalidTransition(message))
            if message.contains("stale")
    ));
}

fn coordinator() -> WorkCoordinator {
    WorkCoordinator::new(Arc::new(MemoryStore::default()))
}

fn verification_input(run: &WorkRun, target_tree: &str, final_tree: &str) -> WorkVerificationInput {
    let attempt = &run.attempts[&WorkAttemptId::new("attempt").unwrap()];
    let result = attempt.result.as_ref().unwrap();
    let contract = run
        .contract(&attempt.contract.contract_id, attempt.contract.revision)
        .unwrap();
    let roots = attempt
        .roots
        .iter()
        .map(|root| {
            let RootState::Git { repositories } = &root.state else {
                panic!("test contract uses a Git root")
            };
            VerificationRoot {
                source_dir_id: root.dir_id.clone(),
                checkpoint_digest: super::root_checkpoint_digest(root).unwrap(),
                state: VerificationRootState::Git {
                    repositories: repositories
                        .iter()
                        .map(|repository| GitVerificationRepository {
                            repository_id: repository.repository_id.clone(),
                            relative_path: repository.relative_path.clone(),
                            target: repository.target.clone(),
                            target_tree: target_tree.into(),
                            final_tree: final_tree.into(),
                        })
                        .collect(),
                },
            }
        })
        .collect();
    let ordered_results = vec![super::WorkResultRef {
        attempt_id: attempt.attempt_id.clone(),
        result_digest: result.result_digest.clone(),
    }];
    WorkVerificationInput {
        goal_revision: run.current_goal().unwrap().revision,
        topology_revision: run.topology_revision,
        coordination_digest: super::verification_coordination_digest(run, &ordered_results)
            .unwrap(),
        ordered_results,
        ordered_change_sets: result
            .change_set_ids
            .iter()
            .map(|change_set_id| VerificationChangeSetInput {
                attempt_id: attempt.attempt_id.clone(),
                change_set: super::WorkAttemptChangeEvidenceRef {
                    change_set_id: change_set_id.clone(),
                    evidence_digest: digest(&format!("evidence-{change_set_id}")),
                },
            })
            .collect(),
        serializability: WorkSerializabilityEvidence {
            status: WorkSerializabilityStatus::Proven,
            evidence_digest: digest("serializability-proof-v1"),
            reason: "declared and actual dependencies form one stable order".into(),
        },
        roots,
        authorization_digests: BTreeSet::from([
            contract.authorization.grant_set_digest.clone(),
            contract.authorization.granted_effects_digest.clone(),
        ]),
        control_resource_digests: attempt
            .roots
            .iter()
            .flat_map(|root| {
                root.control_resources
                    .iter()
                    .map(|resource| resource.content_digest.clone())
            })
            .collect(),
        validation_profile_digests: BTreeSet::from([contract
            .validation_profile
            .content_digest
            .clone()]),
        validator_digest: digest("validator-v1"),
        environment_digest: digest("verification-environment-v1"),
    }
}

fn create_request(
    command_id: &str,
    work_run_id: &str,
    root_participant: WorkParticipant,
) -> WorkRunCommandRequest {
    request(
        command_id,
        work_run_id,
        0,
        WorkRunCommand::Create {
            objective: "deliver reliable work".into(),
            acceptance_conditions: vec!["all checks pass".into()],
            exclusions: vec!["no implicit authority".into()],
            root_participant,
        },
    )
}

fn request(
    command_id: &str,
    work_run_id: &str,
    expected_revision: u64,
    command: WorkRunCommand,
) -> WorkRunCommandRequest {
    WorkRunCommandRequest {
        command_id: CommandId::new(command_id).unwrap(),
        work_run_id: WorkRunId::new(work_run_id).unwrap(),
        expected_revision,
        command,
    }
}

fn participant(session_id: &str, thread_id: &str) -> WorkParticipant {
    WorkParticipant {
        session_id: SessionId::new(session_id).unwrap(),
        thread_id: ThreadId::new(thread_id).unwrap(),
        relation: WorkParticipantRelation::Root,
    }
}

fn contract(contract_id: &str, owner_thread_id: &str) -> WorkContractDraft {
    WorkContractDraft {
        contract_id: WorkContractId::new(contract_id).unwrap(),
        goal_revision: 1,
        topology_revision: 1,
        owner_thread_id: ThreadId::new(owner_thread_id).unwrap(),
        objective: "implement bounded work".into(),
        acceptance_conditions: vec!["target behavior passes".into()],
        exclusions: Vec::new(),
        environment_id: EnvId::local(),
        roots: vec![RootCheckpoint {
            environment_id: EnvId::local(),
            dir_id: DirId::from_str(&format!("sha256:{}", "a".repeat(64))).unwrap(),
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
        primary_root_dir_id: DirId::from_str(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        authorization: AuthorizationSnapshotRef {
            authority: "permission-authority".into(),
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

fn contract_at_topology(
    contract_id: &str,
    owner_thread_id: &str,
    topology_revision: u64,
) -> WorkContractDraft {
    let mut contract = contract(contract_id, owner_thread_id);
    contract.topology_revision = topology_revision;
    contract
}

fn create_run_contract_attempt(
    coordinator: &WorkCoordinator,
    work_run_id: &str,
    owner_thread_id: &str,
    attempt_id: &str,
) {
    coordinator
        .apply(create_request(
            &format!("create-{work_run_id}"),
            work_run_id,
            participant("session-owner", owner_thread_id),
        ))
        .unwrap();
    coordinator
        .apply(request(
            &format!("contract-{work_run_id}"),
            work_run_id,
            1,
            WorkRunCommand::CreateContract {
                contract: contract("contract-owner", owner_thread_id),
            },
        ))
        .unwrap();
    create_attempt(
        coordinator,
        work_run_id,
        2,
        attempt_id,
        "contract-owner",
        owner_thread_id,
    );
}

fn create_attempt(
    coordinator: &WorkCoordinator,
    work_run_id: &str,
    expected_revision: u64,
    attempt_id: &str,
    contract_id: &str,
    thread_id: &str,
) {
    create_planned_attempt(
        coordinator,
        work_run_id,
        expected_revision,
        attempt_id,
        contract_id,
        thread_id,
    );
    let attempt_id = WorkAttemptId::new(attempt_id).unwrap();
    mark_workspace_ready(coordinator, work_run_id, expected_revision + 1, &attempt_id);
}

fn create_planned_attempt(
    coordinator: &WorkCoordinator,
    work_run_id: &str,
    expected_revision: u64,
    attempt_id: &str,
    contract_id: &str,
    thread_id: &str,
) {
    coordinator
        .apply(request(
            &format!("create-{attempt_id}"),
            work_run_id,
            expected_revision,
            WorkRunCommand::CreateAttempt {
                attempt_id: WorkAttemptId::new(attempt_id).unwrap(),
                contract: WorkContractRef {
                    contract_id: WorkContractId::new(contract_id).unwrap(),
                    revision: 1,
                },
                participant_thread_id: ThreadId::new(thread_id).unwrap(),
            },
        ))
        .unwrap();
}

fn mark_workspace_ready(
    coordinator: &WorkCoordinator,
    work_run_id: &str,
    expected_revision: u64,
    attempt_id: &WorkAttemptId,
) {
    let run = coordinator
        .read(&WorkRunId::new(work_run_id).unwrap())
        .unwrap();
    let roots = managed_roots(&run, attempt_id);
    coordinator
        .apply(request(
            &format!("ready-{attempt_id}"),
            work_run_id,
            expected_revision,
            WorkRunCommand::RecordAttemptWorkspaceReady {
                attempt_id: attempt_id.clone(),
                roots,
                private_output_dir_id: test_dir(&format!("output-{attempt_id}")),
            },
        ))
        .unwrap();
}

fn managed_roots(run: &WorkRun, attempt_id: &WorkAttemptId) -> Vec<super::ManagedRootBinding> {
    run.attempts[attempt_id]
        .roots
        .iter()
        .enumerate()
        .map(|(index, checkpoint)| super::ManagedRootBinding {
            source_dir_id: checkpoint.dir_id.clone(),
            managed_dir_id: test_dir(&format!("managed-{attempt_id}-{index}")),
            root_checkpoint_digest: crate::validation::root_checkpoint_digest(checkpoint).unwrap(),
            binding_manifest_digest: digest(&format!("binding-{attempt_id}-{index}")),
        })
        .collect()
}

fn test_dir(seed: &str) -> DirId {
    let digest = ContentDigest::sha256(seed.as_bytes());
    DirId::from_str(digest.as_str()).unwrap()
}

fn begin_attempt(
    coordinator: &WorkCoordinator,
    work_run_id: &str,
    expected_revision: u64,
    attempt_id: &str,
    execution_id: &str,
) {
    coordinator
        .apply(request(
            &format!("begin-{attempt_id}"),
            work_run_id,
            expected_revision,
            WorkRunCommand::BeginAttempt {
                attempt_id: WorkAttemptId::new(attempt_id).unwrap(),
                execution_id: WorkExecutionId::new(execution_id).unwrap(),
                mode: WorkStartMode::Write,
            },
        ))
        .unwrap();
}

fn digest(value: &str) -> ContentDigest {
    ContentDigest::sha256(value.as_bytes())
}
