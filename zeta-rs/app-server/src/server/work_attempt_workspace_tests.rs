use super::AppServer;
use super::work_attempt_effects::work_attempt_effects;
use crate::local::LocalAppServerOptions;
use crate::local::open_local_app_server;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use zeta_core::StartThreadRequest;
use zeta_core::TurnExecutionFinished;
use zeta_core::TurnExecutionKind;
use zeta_core::TurnExecutionObserver;
use zeta_core::TurnExecutionStarted;
use zeta_core::TurnExecutionTerminalState;
use zeta_core::TurnToolExecutionFinished;
use zeta_core::TurnToolExecutionStarted;
use zeta_file_access::Dir;
use zeta_file_access::Grant;
use zeta_file_access::GrantSource;
use zeta_file_access::Permission;
use zeta_file_access::Permissions;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::TurnId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_turn_changes::CaptureState;
use zeta_turn_changes::TerminalTurnState;
use zeta_turn_changes::TurnChangeStore;
use zeta_work_coordination::AuthorizationSnapshotRef;
use zeta_work_coordination::ExternalEffectsStatus;
use zeta_work_coordination::GitRepositoryCheckpoint;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::IntegrationFailureKind;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::ValidationProfileRef;
use zeta_work_coordination::VerificationCheckEvidence;
use zeta_work_coordination::VerificationCheckOutcome;
use zeta_work_coordination::VerificationConclusion;
use zeta_work_coordination::WorkAttemptChangeEvidenceRef;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkAttemptIntegrationStatus;
use zeta_work_coordination::WorkAttemptVerificationStatus;
use zeta_work_coordination::WorkCommandDisposition;
use zeta_work_coordination::WorkContractDraft;
use zeta_work_coordination::WorkIntegrationStatus;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkStartMode;
use zeta_work_coordination::WorkVerificationStatus;
use zeta_work_coordination::integration_key;
use zeta_work_coordination::verification_key;
use zeta_work_coordination::work_attempt_result_digest;

#[test]
fn work_attempt_recovers_and_captures_every_root_with_exact_provenance() {
    let profile = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    initialize_repository(first.path(), "first.txt");
    initialize_repository(second.path(), "second.txt");

    let first_dir = Dir::open_local(first.path()).unwrap();
    let second_dir = Dir::open_local(second.path()).unwrap();
    let roots = vec![root_checkpoint(&first_dir), root_checkpoint(&second_dir)];
    let primary_root_dir_id = first_dir.id();
    let server = open_server(profile.path(), first.path());
    let thread = server
        .threads
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("start-work-attempt-thread").unwrap(),
            title: "multi-root attempt".into(),
        })
        .unwrap();
    grant_session_root(&server, &thread.session_id, second_dir.clone());

    let work_run_id = WorkRunId::new("multi-root-work-run").unwrap();
    let contract_id = WorkContractId::new("multi-root-contract").unwrap();
    let attempt_id = WorkAttemptId::new("multi-root-attempt").unwrap();
    let execution_id = WorkExecutionId::new("multi-root-execution").unwrap();
    let runtime = server.work_coordination.as_ref().unwrap();
    let mut run = apply(
        runtime,
        &work_run_id,
        0,
        "create-multi-root-run",
        WorkRunCommand::Create {
            objective: "change two repositories in one exact attempt".into(),
            acceptance_conditions: vec!["both roots are captured".into()],
            exclusions: Vec::new(),
            root_participant: WorkParticipant {
                session_id: thread.session_id.clone(),
                thread_id: thread.thread_id.clone(),
                relation: WorkParticipantRelation::Root,
            },
        },
    );
    run = apply(
        runtime,
        &work_run_id,
        run.revision,
        "create-multi-root-contract",
        WorkRunCommand::CreateContract {
            contract: WorkContractDraft {
                contract_id: contract_id.clone(),
                goal_revision: 1,
                topology_revision: 1,
                owner_thread_id: thread.thread_id.clone(),
                objective: "change both roots".into(),
                acceptance_conditions: vec!["both roots are captured".into()],
                exclusions: Vec::new(),
                environment_id: first_dir.env().clone(),
                roots,
                primary_root_dir_id: primary_root_dir_id.clone(),
                authorization: AuthorizationSnapshotRef {
                    authority: "test-host".into(),
                    policy_revision: "test-policy-v1".into(),
                    grant_set_digest: ContentDigest::sha256(b"test-grants"),
                    granted_effects_digest: ContentDigest::sha256(b"test-effects"),
                },
                decision_ids: BTreeSet::new(),
                upstream_results: Vec::new(),
                expected_scope: WorkScopeClaim::default(),
                validation_profile: ValidationProfileRef {
                    name: "test-profile".into(),
                    content_digest: ContentDigest::sha256(b"test-profile"),
                },
            },
        },
    );
    run = apply(
        runtime,
        &work_run_id,
        run.revision,
        "create-multi-root-attempt",
        WorkRunCommand::CreateAttempt {
            attempt_id: attempt_id.clone(),
            contract: zeta_work_coordination::WorkContractRef {
                contract_id: contract_id.clone(),
                revision: 1,
            },
            participant_thread_id: thread.thread_id.clone(),
        },
    );
    let ready = run.attempts.get(&attempt_id).unwrap();
    let expected_workspace = ready.workspace.clone();
    apply(
        runtime,
        &work_run_id,
        run.revision,
        "begin-multi-root-attempt",
        WorkRunCommand::BeginAttempt {
            attempt_id: attempt_id.clone(),
            execution_id: execution_id.clone(),
            mode: WorkStartMode::Write,
        },
    );
    assert!(
        server
            .env_runtime
            .read()
            .unwrap()
            .dir_grants
            .thread_scope(&thread.thread_id, Permission::InspectRepository)
            .unwrap()
            .unwrap()
            .is_exact()
    );

    drop(server);

    let server = open_server(profile.path(), first.path());
    let recovered = server
        .work_coordination
        .as_ref()
        .unwrap()
        .read(&work_run_id)
        .unwrap();
    assert_eq!(
        recovered.attempts.get(&attempt_id).unwrap().workspace,
        expected_workspace
    );
    let changes = server.turn_changes.as_ref().unwrap().clone();
    let execution_roots = changes.execution_roots(&thread.thread_id).unwrap();
    assert_eq!(execution_roots.len(), 2);
    assert!(execution_roots.iter().any(|root| root.primary));
    assert!(execution_roots.iter().all(|root| {
        root.work_attempt.as_ref().is_some_and(|provenance| {
            provenance.work_run_id == work_run_id
                && provenance.attempt_id == attempt_id
                && provenance.execution_id == execution_id
                && provenance.contract_id == contract_id
                && provenance.contract_revision == 1
        })
    }));

    let turn_id = TurnId::new("multi-root-turn").unwrap();
    changes
        .will_execute(&TurnExecutionStarted {
            session_id: thread.session_id.clone(),
            thread_id: thread.thread_id.clone(),
            turn_id: turn_id.clone(),
            kind: TurnExecutionKind::Agent,
        })
        .unwrap();
    for (index, root) in execution_roots.iter().enumerate() {
        let file_name = format!("attempt-{index}.txt");
        let source_path = root.source.canonical_path().join(&file_name);
        let tool_call_id = ToolCallId::new(format!("write-root-{index}")).unwrap();
        let arguments = serde_json::json!({"path": source_path});
        changes
            .tool_will_execute(&TurnToolExecutionStarted {
                session_id: thread.session_id.clone(),
                thread_id: thread.thread_id.clone(),
                turn_id: turn_id.clone(),
                tool_call_id: tool_call_id.clone(),
                name: ToolName::new("write_file").unwrap(),
                arguments: arguments.clone(),
                write_capable: true,
            })
            .unwrap();
        std::fs::write(
            root.binding.dir().join(&file_name),
            format!("root {index}\n"),
        )
        .unwrap();
        changes.tool_did_finish(&TurnToolExecutionFinished {
            session_id: thread.session_id.clone(),
            thread_id: thread.thread_id.clone(),
            turn_id: turn_id.clone(),
            tool_call_id,
            name: ToolName::new("write_file").unwrap(),
            arguments,
            outcome_unknown: false,
        });
    }
    changes.did_finish(&TurnExecutionFinished {
        session_id: thread.session_id.clone(),
        thread_id: thread.thread_id.clone(),
        turn_id,
        terminal_state: TurnExecutionTerminalState::Completed,
    });

    let records = changes.store.list_for_thread(&thread.thread_id).unwrap();
    assert_eq!(records.len(), 2);
    assert!(
        records.iter().all(|record| {
            record.capture_state == CaptureState::Sealed
                && record.terminal_state == Some(TerminalTurnState::Completed)
                && !record.attribution_incomplete
                && record.files.len() == 1
                && record.write_paths.len() == 1
                && record.work_attempt.as_ref().is_some_and(|provenance| {
                    provenance.work_run_id == work_run_id
                        && provenance.attempt_id == attempt_id
                        && provenance.execution_id == execution_id
                })
        }),
        "records: {records:#?}"
    );
    let direct_commit = changes.queue_commit(
        records[0].clone(),
        records[0].revision,
        &CommandId::new("bypass-work-attempt-integration").unwrap(),
        "bypass-work-attempt-integration",
    );
    assert_eq!(
        direct_commit.unwrap_err(),
        "a WorkAttempt ChangeSet can be published only by the integration gate"
    );
    assert!(!first.path().join("attempt-0.txt").exists());
    assert!(!second.path().join("attempt-1.txt").exists());

    let bindings = changes
        .work_attempt_bindings
        .read()
        .unwrap()
        .get(&(work_run_id.clone(), attempt_id.clone()))
        .cloned()
        .unwrap();
    std::fs::write(
        bindings.output.root().join("artifact.txt"),
        "private output\n",
    )
    .unwrap();
    let private_output_digest = changes.worktrees.capture_output(&bindings.output).unwrap();
    let thread = server.threads.read_thread(&thread.thread_id).unwrap();
    let effect_turn_ids = records
        .iter()
        .map(|record| record.turn_id.clone())
        .collect::<BTreeSet<_>>();
    let effects = work_attempt_effects(&thread, &effect_turn_ids).unwrap();
    assert_eq!(effects.status, ExternalEffectsStatus::None);
    let external_effects_digest = effects.digest;
    let evidence = records
        .iter()
        .map(|record| WorkAttemptChangeEvidenceRef {
            change_set_id: record.change_set_id.clone(),
            evidence_digest: record.evidence_digest().unwrap(),
        })
        .collect::<Vec<_>>();
    let attempt = recovered.attempts.get(&attempt_id).unwrap();
    let result_digest = work_attempt_result_digest(
        &work_run_id,
        recovered.topology_revision,
        attempt,
        &evidence,
        &private_output_digest,
        &external_effects_digest,
        ExternalEffectsStatus::None,
    )
    .unwrap();
    let change_set_ids = records
        .iter()
        .map(|record| record.change_set_id.clone())
        .collect::<Vec<_>>();

    let rogue = execution_roots[0].binding.dir().join("unsealed.txt");
    std::fs::write(&rogue, "not in a ChangeSet\n").unwrap();
    let rejected = server
        .work_coordination
        .as_ref()
        .unwrap()
        .apply(WorkRunCommandRequest {
            command_id: CommandId::new("reject-unsealed-root-state").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: recovered.revision,
            command: WorkRunCommand::SealAttempt {
                attempt_id: attempt_id.clone(),
                result_digest: result_digest.clone(),
                change_set_ids: change_set_ids.clone(),
                private_output_digest: private_output_digest.clone(),
                external_effects_digest: external_effects_digest.clone(),
                external_effects_status: ExternalEffectsStatus::None,
            },
        });
    let rejected = rejected.unwrap_err().to_string();
    assert!(rejected.contains("changed after"), "rejected: {rejected}");
    std::fs::remove_file(rogue).unwrap();

    let seal_request = WorkRunCommandRequest {
        command_id: CommandId::new("seal-multi-root-attempt").unwrap(),
        work_run_id: work_run_id.clone(),
        expected_revision: recovered.revision,
        command: WorkRunCommand::SealAttempt {
            attempt_id: attempt_id.clone(),
            result_digest,
            change_set_ids,
            private_output_digest,
            external_effects_digest,
            external_effects_status: ExternalEffectsStatus::None,
        },
    };
    let sealed = server
        .work_coordination
        .as_ref()
        .unwrap()
        .apply(seal_request.clone())
        .unwrap();
    assert_eq!(sealed.disposition, WorkCommandDisposition::Committed);
    assert_eq!(
        server
            .work_coordination
            .as_ref()
            .unwrap()
            .read(&work_run_id)
            .unwrap()
            .attempts[&attempt_id]
            .execution_status,
        WorkAttemptExecutionStatus::Sealed
    );
    assert_eq!(
        server
            .work_coordination
            .as_ref()
            .unwrap()
            .apply(seal_request)
            .unwrap()
            .disposition,
        WorkCommandDisposition::Replayed
    );
    let sealed_run = server
        .work_coordination
        .as_ref()
        .unwrap()
        .read(&work_run_id)
        .unwrap();
    let verification_revision = sealed_run.revision;
    let verification = server
        .work_coordination
        .as_ref()
        .unwrap()
        .request_verification(
            CommandId::new("verify-multi-root-attempt").unwrap(),
            work_run_id.clone(),
            verification_revision,
            BTreeSet::from([attempt_id.clone()]),
        )
        .unwrap();
    assert_eq!(verification.disposition, WorkCommandDisposition::Committed);
    assert_eq!(verification.work_run.verifications.len(), 1);
    let evidence = verification.work_run.verifications.values().next().unwrap();
    assert_eq!(evidence.status, WorkVerificationStatus::Indeterminate);
    assert_eq!(evidence.input.roots.len(), 2);
    assert!(evidence.checks.iter().any(|check| {
        check.check_id == "immutable-replay"
            && check.outcome == zeta_work_coordination::VerificationCheckOutcome::Passed
    }));
    assert!(evidence.checks.iter().any(|check| {
        check.check_id == "acceptance-profile"
            && check.outcome == zeta_work_coordination::VerificationCheckOutcome::Indeterminate
    }));
    assert_eq!(
        verification.work_run.attempts[&attempt_id].verification_status,
        WorkAttemptVerificationStatus::Indeterminate
    );
    assert!(!first.path().join("attempt-0.txt").exists());
    assert!(!second.path().join("attempt-1.txt").exists());
    let replayed_verification = server
        .work_coordination
        .as_ref()
        .unwrap()
        .request_verification(
            CommandId::new("verify-multi-root-attempt").unwrap(),
            work_run_id.clone(),
            verification_revision,
            BTreeSet::from([attempt_id.clone()]),
        )
        .unwrap();
    assert_eq!(
        replayed_verification.disposition,
        WorkCommandDisposition::Replayed
    );
    assert_eq!(replayed_verification.work_run, verification.work_run);

    let indeterminate_key = evidence.verification_key.clone();
    let stale = server
        .work_coordination
        .as_ref()
        .unwrap()
        .apply_state_for_test(WorkRunCommandRequest {
            command_id: CommandId::new("stale-unqualified-verification").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: replayed_verification.work_run.revision,
            command: WorkRunCommand::MarkVerificationStale {
                verification_key: indeterminate_key,
                reason: "test installs an independently qualified validator record".into(),
            },
        })
        .unwrap()
        .work_run;
    let mut qualified_input = changes
        .prepare_work_verification(&stale, &BTreeSet::from([attempt_id.clone()]))
        .unwrap();
    qualified_input.validator_digest = ContentDigest::sha256(b"qualified-test-validator");
    let qualified_key = verification_key(&work_run_id, &qualified_input).unwrap();
    let verifying = server
        .work_coordination
        .as_ref()
        .unwrap()
        .apply_state_for_test(WorkRunCommandRequest {
            command_id: CommandId::new("begin-qualified-test-verification").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: stale.revision,
            command: WorkRunCommand::BeginVerification {
                input: qualified_input,
            },
        })
        .unwrap()
        .work_run;
    let verified = server
        .work_coordination
        .as_ref()
        .unwrap()
        .apply_state_for_test(WorkRunCommandRequest {
            command_id: CommandId::new("finish-qualified-test-verification").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: verifying.revision,
            command: WorkRunCommand::FinishVerification {
                verification_key: qualified_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![VerificationCheckEvidence {
                    check_id: "qualified-test-validator".into(),
                    command_digest: ContentDigest::sha256(b"qualified-command"),
                    output_digest: ContentDigest::sha256(b"qualified-output"),
                    outcome: VerificationCheckOutcome::Passed,
                }],
                reason: "independent test validator passed".into(),
            },
        })
        .unwrap()
        .work_run;
    let moved_root_dir_id = verified.verifications[&qualified_key]
        .input
        .roots
        .last()
        .unwrap()
        .source_dir_id
        .clone();
    let (moved_path, stable_path) = if moved_root_dir_id == first_dir.id() {
        (first.path(), second.path())
    } else {
        (second.path(), first.path())
    };
    let stable_head_before_conflict = run_git(stable_path, &["rev-parse", "HEAD"]);
    std::fs::write(moved_path.join("concurrent.txt"), "concurrent change\n").unwrap();
    run_git(moved_path, &["add", "concurrent.txt"]);
    run_git(
        moved_path,
        &["commit", "--quiet", "-m", "concurrent target movement"],
    );
    let moved_head = run_git(moved_path, &["rev-parse", "HEAD"]);
    let conflicted = server
        .work_coordination
        .as_ref()
        .unwrap()
        .request_integration(
            CommandId::new("reject-moved-multi-root-target").unwrap(),
            work_run_id.clone(),
            verified.revision,
            qualified_key,
        )
        .unwrap()
        .work_run;
    let conflicted_integration = conflicted.integrations.values().next().unwrap();
    assert_eq!(
        conflicted_integration.status,
        WorkIntegrationStatus::Conflict
    );
    assert_eq!(
        conflicted_integration.incidents.last().unwrap().kind,
        IntegrationFailureKind::TargetMoved
    );
    assert_eq!(
        conflicted_integration
            .incidents
            .last()
            .unwrap()
            .published_root_count,
        0
    );
    assert_eq!(
        conflicted_integration.roots[0].status,
        zeta_work_coordination::IntegrationRootStatus::Prepared
    );
    assert_eq!(
        conflicted_integration.roots[1].status,
        zeta_work_coordination::IntegrationRootStatus::Pending
    );
    assert_eq!(
        run_git(stable_path, &["rev-parse", "HEAD"]),
        stable_head_before_conflict
    );
    assert_eq!(run_git(moved_path, &["rev-parse", "HEAD"]), moved_head);
    assert!(!first.path().join("attempt-0.txt").exists());
    assert!(!second.path().join("attempt-1.txt").exists());
    assert_eq!(
        conflicted.verifications[&conflicted_integration.verification_key].status,
        WorkVerificationStatus::Stale
    );

    let mut moved_target_input = changes
        .prepare_work_verification(&conflicted, &BTreeSet::from([attempt_id.clone()]))
        .unwrap();
    moved_target_input.validator_digest =
        ContentDigest::sha256(b"qualified-test-validator-after-target-movement");
    let moved_target_key = verification_key(&work_run_id, &moved_target_input).unwrap();
    let reverifying = server
        .work_coordination
        .as_ref()
        .unwrap()
        .apply_state_for_test(WorkRunCommandRequest {
            command_id: CommandId::new("begin-moved-target-test-verification").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: conflicted.revision,
            command: WorkRunCommand::BeginVerification {
                input: moved_target_input,
            },
        })
        .unwrap()
        .work_run;
    let reverified = server
        .work_coordination
        .as_ref()
        .unwrap()
        .apply_state_for_test(WorkRunCommandRequest {
            command_id: CommandId::new("finish-moved-target-test-verification").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: reverifying.revision,
            command: WorkRunCommand::FinishVerification {
                verification_key: moved_target_key.clone(),
                conclusion: VerificationConclusion::Verified,
                checks: vec![VerificationCheckEvidence {
                    check_id: "qualified-test-validator".into(),
                    command_digest: ContentDigest::sha256(b"qualified-moved-target-command"),
                    output_digest: ContentDigest::sha256(b"qualified-moved-target-output"),
                    outcome: VerificationCheckOutcome::Passed,
                }],
                reason: "independent test validator passed against the moved target".into(),
            },
        })
        .unwrap()
        .work_run;
    let integration_revision = reverified.revision;
    let integration_key = integration_key(&work_run_id, &moved_target_key).unwrap();
    let runtime = server.work_coordination.as_ref().unwrap();
    let queued = runtime
        .apply_state_for_test(WorkRunCommandRequest {
            command_id: CommandId::new("integrate-qualified-multi-root-result").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: integration_revision,
            command: WorkRunCommand::QueueIntegration {
                verification_key: moved_target_key.clone(),
            },
        })
        .unwrap();
    assert_eq!(queued.disposition, WorkCommandDisposition::Committed);
    let mut publishing = queued.work_run;
    let root_count = publishing.integrations[&integration_key].roots.len();
    for index in 0..root_count {
        let integration = publishing.integrations[&integration_key].clone();
        let root = integration.roots[index].clone();
        let artifact = changes
            .prepare_work_integration_root(&publishing, &integration, &root)
            .unwrap();
        publishing = runtime
            .apply_state_for_test(WorkRunCommandRequest {
                command_id: CommandId::new(format!("prepare-crash-root-{index}")).unwrap(),
                work_run_id: work_run_id.clone(),
                expected_revision: publishing.revision,
                command: WorkRunCommand::RecordIntegrationRootPrepared {
                    integration_key: integration_key.clone(),
                    generation: integration.generation,
                    root_id: root.root_id,
                    artifact,
                },
            })
            .unwrap()
            .work_run;
    }
    let generation = publishing.integrations[&integration_key].generation;
    publishing = runtime
        .apply_state_for_test(WorkRunCommandRequest {
            command_id: CommandId::new("begin-crash-boundary-publication").unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision: publishing.revision,
            command: WorkRunCommand::BeginIntegration {
                integration_key: integration_key.clone(),
                generation,
            },
        })
        .unwrap()
        .work_run;
    let physical_integration = publishing.integrations[&integration_key].clone();
    let physical_root = physical_integration.roots[0].clone();
    let (published_path, waiting_path) = if physical_root.source_dir_id == first_dir.id() {
        (first.path(), second.path())
    } else {
        (second.path(), first.path())
    };
    let published_head_before_crash = run_git(published_path, &["rev-parse", "HEAD"]);
    let waiting_head_before_crash = run_git(waiting_path, &["rev-parse", "HEAD"]);
    let _unrecorded_receipt = changes
        .publish_work_integration_root(&publishing, &physical_integration, &physical_root)
        .unwrap();
    assert_ne!(
        run_git(published_path, &["rev-parse", "HEAD"]),
        published_head_before_crash
    );
    assert_eq!(
        run_git(waiting_path, &["rev-parse", "HEAD"]),
        waiting_head_before_crash
    );
    assert_eq!(
        publishing.integrations[&integration_key].roots[0].status,
        zeta_work_coordination::IntegrationRootStatus::Prepared
    );

    drop(changes);
    drop(server);

    let server = open_server(profile.path(), first.path());
    let integrated_run = server
        .work_coordination
        .as_ref()
        .unwrap()
        .read(&work_run_id)
        .unwrap();
    assert_eq!(integrated_run.integrations.len(), 2);
    let integration = &integrated_run.integrations[&integration_key];
    assert_eq!(integration.status, WorkIntegrationStatus::Integrated);
    assert!(integration.roots.iter().all(|root| {
        root.status == zeta_work_coordination::IntegrationRootStatus::Published
            && root.prepared_artifact.is_some()
            && root.publication_receipt_digest.is_some()
    }));
    assert_eq!(
        integrated_run.attempts[&attempt_id].integration_status,
        WorkAttemptIntegrationStatus::Integrated
    );
    assert_eq!(
        std::fs::read_to_string(first.path().join("attempt-0.txt")).unwrap(),
        "root 0\n"
    );
    assert_eq!(
        std::fs::read_to_string(second.path().join("attempt-1.txt")).unwrap(),
        "root 1\n"
    );
    assert_eq!(
        server
            .work_coordination
            .as_ref()
            .unwrap()
            .request_integration(
                CommandId::new("integrate-qualified-multi-root-result").unwrap(),
                work_run_id.clone(),
                integration_revision,
                moved_target_key,
            )
            .unwrap()
            .disposition,
        WorkCommandDisposition::Replayed
    );
    assert!(
        !server
            .env_runtime
            .read()
            .unwrap()
            .dir_grants
            .thread_scope(&thread.thread_id, Permission::InspectRepository)
            .unwrap()
            .unwrap()
            .is_exact()
    );
}

fn open_server(profile: &Path, root: &Path) -> AppServer {
    open_local_app_server(
        LocalAppServerOptions::new(profile)
            .without_built_in_skills()
            .with_dir_root(root),
    )
    .unwrap()
}

fn grant_session_root(server: &AppServer, session_id: &zeta_protocol::SessionId, dir: Dir) {
    let grants = server.env_runtime.read().unwrap().dir_grants.clone();
    grants
        .add_dir(
            session_id.clone(),
            Grant::for_session_tree(
                session_id.clone(),
                dir,
                GrantSource::HostConfiguration,
                Permissions::new([
                    Permission::ExecuteCommands,
                    Permission::InspectRepository,
                    Permission::MutateRepository,
                ]),
            ),
        )
        .unwrap();
}

fn apply(
    runtime: &super::work_coordination_runtime::WorkCoordinationRuntime,
    work_run_id: &WorkRunId,
    expected_revision: u64,
    command_id: &str,
    command: WorkRunCommand,
) -> WorkRun {
    runtime
        .apply(WorkRunCommandRequest {
            command_id: CommandId::new(command_id).unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision,
            command,
        })
        .unwrap();
    runtime.read(work_run_id).unwrap()
}

fn initialize_repository(root: &Path, initial_file: &str) {
    run_git(root, &["init", "--quiet", "--initial-branch=main"]);
    run_git(root, &["config", "user.name", "Zeta WorkAttempt Test"]);
    run_git(
        root,
        &["config", "user.email", "zeta-work-attempt@example.invalid"],
    );
    std::fs::write(root.join(initial_file), "initial\n").unwrap();
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "--quiet", "-m", "initial"]);
}

fn root_checkpoint(dir: &Dir) -> RootCheckpoint {
    let head = run_git(dir.canonical_path(), &["rev-parse", "HEAD"]);
    let tree = run_git(dir.canonical_path(), &["rev-parse", "HEAD^{tree}"]);
    let common_dir = Dir::open_local(dir.canonical_path().join(".git")).unwrap();
    RootCheckpoint {
        environment_id: dir.env().clone(),
        dir_id: dir.id(),
        state: RootState::Git {
            repositories: vec![GitRepositoryCheckpoint {
                repository_id: format!("git:{}", common_dir.id()),
                relative_path: ".".into(),
                target: GitRootTarget::Branch {
                    name: "main".into(),
                    expected_head: head,
                },
                baseline_tree: tree,
            }],
        },
        control_resources: Vec::new(),
    }
}

fn run_git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-c")
        .arg("core.hooksPath=/dev/null")
        .args(arguments)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
