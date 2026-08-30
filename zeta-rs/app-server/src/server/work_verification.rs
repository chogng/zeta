use super::turn_changes_runtime::TurnChangesRuntime;
use super::work_attempt_effects::work_attempt_effects;
use super::work_attempt_workspace::WorkAttemptRootBinding;
use super::work_serializability::analyze_work_serializability;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use zeta_file_access::Dir;
use zeta_file_access::DirId;
use zeta_git::GitClient;
use zeta_git::GitTreeId;
use zeta_git::GitTreeReplayResult;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkAttemptId;
use zeta_turn_changes::CaptureState;
use zeta_turn_changes::CommitState;
use zeta_turn_changes::DirectoryReplayResult;
use zeta_turn_changes::DirectorySnapshotStore;
use zeta_turn_changes::SnapshotBackend;
use zeta_turn_changes::TerminalTurnState;
use zeta_turn_changes::TurnChangeSet;
use zeta_turn_changes::TurnChangeStore;
use zeta_work_coordination::ExternalEffectsStatus;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::GitVerificationRepository;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::VerificationChangeSetInput;
use zeta_work_coordination::VerificationCheckEvidence;
use zeta_work_coordination::VerificationCheckOutcome;
use zeta_work_coordination::VerificationConclusion;
use zeta_work_coordination::VerificationRoot;
use zeta_work_coordination::VerificationRootState;
use zeta_work_coordination::WorkAttemptChangeEvidenceRef;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkSerializabilityStatus;
use zeta_work_coordination::WorkVerification;
use zeta_work_coordination::WorkVerificationInput;
use zeta_work_coordination::root_checkpoint_digest;
use zeta_work_coordination::verification_coordination_digest;
use zeta_work_coordination::verification_key;
use zeta_work_coordination::work_attempt_result_digest;
use zeta_worktree::ManagedDirBinding;
use zeta_worktree::ManagedDirKind;
use zeta_worktree::ManagedDirOwner;
use zeta_worktree::ManagedDirProvisionRequest;
use zeta_worktree::ManagedDirSource;
use zeta_worktree::ManagedDirTarget;
use zeta_worktree::ManagedOutputOwner;

const VERIFIER_REVISION: &str = "zeta-independent-verifier-v2";

pub(super) struct WorkVerificationExecution {
    pub(super) conclusion: VerificationConclusion,
    pub(super) checks: Vec<VerificationCheckEvidence>,
    pub(super) reason: String,
}

#[derive(Clone)]
struct VerificationSourceRoot {
    checkpoint: RootCheckpoint,
    source: Dir,
    managed: ManagedDirBinding,
}

impl TurnChangesRuntime {
    /// Derives the complete verification input from durable sealed results and current targets.
    pub(super) fn prepare_work_verification(
        &self,
        run: &WorkRun,
        selected_attempt_ids: &BTreeSet<WorkAttemptId>,
    ) -> Result<WorkVerificationInput, String> {
        let declared_results =
            zeta_work_coordination::ordered_result_refs(run, selected_attempt_ids)
                .map_err(|error| error.to_string())?;
        let mut records_by_attempt = BTreeMap::new();
        for result_ref in &declared_results {
            let attempt = &run.attempts[&result_ref.attempt_id];
            let result = attempt
                .result
                .as_ref()
                .ok_or_else(|| "verification selected an unsealed WorkAttempt".to_string())?;
            let bindings = self
                .work_attempt_bindings
                .read()
                .map_err(|_| "WorkAttempt workspace binding lock poisoned".to_string())?
                .get(&(run.work_run_id.clone(), attempt.attempt_id.clone()))
                .cloned()
                .ok_or_else(|| {
                    "sealed WorkAttempt workspace evidence is unavailable".to_string()
                })?;
            let private_output_digest = self
                .worktrees
                .capture_output(&bindings.output)
                .map_err(|error| error.to_string())?;
            if private_output_digest != result.private_output_digest {
                return Err("sealed WorkAttempt private output changed before verification".into());
            }
            let mut evidence = Vec::with_capacity(result.change_set_ids.len());
            let mut attempt_records = Vec::with_capacity(result.change_set_ids.len());
            for change_set_id in &result.change_set_ids {
                let record = self
                    .store
                    .load(change_set_id)
                    .map_err(|error| error.to_string())?;
                validate_verification_change_set(run, attempt, &record)?;
                let evidence_digest = record
                    .evidence_digest()
                    .map_err(|error| error.to_string())?;
                evidence.push(WorkAttemptChangeEvidenceRef {
                    change_set_id: record.change_set_id.clone(),
                    evidence_digest: evidence_digest.clone(),
                });
                attempt_records.push(record.clone());
            }
            let turn_ids = attempt_records
                .iter()
                .map(|record| record.turn_id.clone())
                .collect::<BTreeSet<_>>();
            let thread = self
                .threads
                .read_thread(&attempt.thread_id)
                .map_err(|error| error.to_string())?;
            let effects = work_attempt_effects(&thread, &turn_ids)?;
            if effects.digest != result.external_effects_digest
                || effects.status != result.external_effects_status
            {
                return Err(
                    "sealed WorkAttempt effect evidence changed before verification".into(),
                );
            }
            let rebuilt = work_attempt_result_digest(
                &run.work_run_id,
                run.topology_revision,
                attempt,
                &evidence,
                &private_output_digest,
                &effects.digest,
                effects.status,
            )
            .map_err(|error| error.to_string())?;
            if rebuilt != result.result_digest || rebuilt != result_ref.result_digest {
                return Err("sealed WorkAttempt result evidence is inconsistent".into());
            }
            if records_by_attempt
                .insert(attempt.attempt_id.clone(), attempt_records)
                .is_some()
            {
                return Err("verification repeats a WorkAttempt result".into());
            }
        }
        let serializability =
            analyze_work_serializability(run, selected_attempt_ids, &records_by_attempt)?;
        let ordered_results = serializability.ordered_results;
        let sources = self.verification_source_roots(run, &ordered_results)?;
        let mut ordered_change_sets = Vec::new();
        let mut records = Vec::new();
        for result in &ordered_results {
            let attempt_records = records_by_attempt
                .get(&result.attempt_id)
                .ok_or_else(|| "ordered WorkAttempt lost its ChangeSet evidence".to_string())?;
            for record in attempt_records {
                ordered_change_sets.push(VerificationChangeSetInput {
                    attempt_id: result.attempt_id.clone(),
                    change_set: WorkAttemptChangeEvidenceRef {
                        change_set_id: record.change_set_id.clone(),
                        evidence_digest: record
                            .evidence_digest()
                            .map_err(|error| error.to_string())?,
                    },
                });
                records.push(record.clone());
            }
        }
        validate_change_set_order(&records)?;

        let roots = sources
            .values()
            .map(|source| self.replay_verification_root(source, &records))
            .collect::<Result<Vec<_>, _>>()?;
        let mut authorization_digests = BTreeSet::new();
        let mut control_resource_digests = BTreeSet::new();
        let mut validation_profile_digests = BTreeSet::new();
        let mut environment_ids = BTreeSet::new();
        for result in &ordered_results {
            let attempt = &run.attempts[&result.attempt_id];
            let contract = run
                .contract(&attempt.contract.contract_id, attempt.contract.revision)
                .ok_or_else(|| {
                    "WorkAttempt contract disappeared before verification".to_string()
                })?;
            authorization_digests.insert(contract.authorization.grant_set_digest.clone());
            authorization_digests.insert(contract.authorization.granted_effects_digest.clone());
            validation_profile_digests.insert(contract.validation_profile.content_digest.clone());
            environment_ids.insert(contract.environment_id.clone());
            for root in &attempt.roots {
                control_resource_digests.extend(
                    root.control_resources
                        .iter()
                        .map(|resource| resource.content_digest.clone()),
                );
            }
        }
        let validator_digest =
            canonical_digest(&(1_u32, VERIFIER_REVISION, env!("CARGO_PKG_VERSION")))?;
        let environment_digest = canonical_digest(&(
            1_u32,
            environment_ids,
            std::env::consts::OS,
            std::env::consts::ARCH,
        ))?;
        let input = WorkVerificationInput {
            goal_revision: run.current_goal().map(|goal| goal.revision).unwrap_or(0),
            topology_revision: run.topology_revision,
            coordination_digest: verification_coordination_digest(run, &ordered_results)
                .map_err(|error| error.to_string())?,
            ordered_results,
            ordered_change_sets,
            serializability: serializability.evidence,
            roots,
            authorization_digests,
            control_resource_digests,
            validation_profile_digests,
            validator_digest,
            environment_digest,
        };
        self.ensure_work_verification_workspace(run, &input, &sources)?;
        Ok(input)
    }

    /// Reconstructs the private final roots and produces a conservative independent conclusion.
    pub(super) fn execute_work_verification(
        &self,
        run: &WorkRun,
        verification: &WorkVerification,
    ) -> WorkVerificationExecution {
        let sources = match self.verification_source_roots(run, &verification.input.ordered_results)
        {
            Ok(sources) => sources,
            Err(error) => return indeterminate_execution("source-roots", &error),
        };
        let workspace_digest =
            match self.ensure_work_verification_workspace(run, &verification.input, &sources) {
                Ok(digest) => digest,
                Err(error) => return indeterminate_execution("workspace-recovery", &error),
            };
        let mut checks = vec![VerificationCheckEvidence {
            check_id: "immutable-replay".into(),
            command_digest: verification.input.validator_digest.clone(),
            output_digest: workspace_digest,
            outcome: VerificationCheckOutcome::Passed,
        }];
        let serializable =
            verification.input.serializability.status == WorkSerializabilityStatus::Proven;
        checks.push(VerificationCheckEvidence {
            check_id: "serializability".into(),
            command_digest: digest_text("actual-effect-serializability-v1"),
            output_digest: verification.input.serializability.evidence_digest.clone(),
            outcome: if serializable {
                VerificationCheckOutcome::Passed
            } else {
                VerificationCheckOutcome::Indeterminate
            },
        });
        let unknown_effects = verification.input.ordered_results.iter().any(|result| {
            run.attempts[&result.attempt_id]
                .result
                .as_ref()
                .is_some_and(|result| {
                    result.external_effects_status == ExternalEffectsStatus::Unknown
                })
        });
        if unknown_effects {
            checks.push(VerificationCheckEvidence {
                check_id: "external-effects".into(),
                command_digest: digest_text("external-effect-reconciliation-v1"),
                output_digest: digest_text("one or more external effects have no trusted receipt"),
                outcome: VerificationCheckOutcome::Indeterminate,
            });
        }
        checks.push(VerificationCheckEvidence {
            check_id: "acceptance-profile".into(),
            command_digest: digest_set(&verification.input.validation_profile_digests),
            output_digest: digest_text(
                "trusted acceptance-profile execution is not qualified for this platform",
            ),
            outcome: VerificationCheckOutcome::Indeterminate,
        });
        WorkVerificationExecution {
            conclusion: VerificationConclusion::Indeterminate,
            checks,
            reason: if !serializable {
                format!(
                    "immutable replay passed, but serializability is not proven: {}",
                    verification.input.serializability.reason
                )
            } else if unknown_effects {
                "immutable replay passed, but external effects and the acceptance validator are not fully qualified"
                    .into()
            } else {
                "immutable replay passed, but the acceptance validator is not fully qualified"
                    .into()
            },
        }
    }

    /// Executes the feature-gated exact-file acceptance profile against the independently
    /// materialized final root. Production compositions never select this profile.
    #[cfg(feature = "multi-agent-evals")]
    pub(super) fn execute_evaluation_verification(
        &self,
        run: &WorkRun,
        verification: &WorkVerification,
        expected_files: &BTreeMap<String, Vec<u8>>,
    ) -> WorkVerificationExecution {
        let sources = match self.verification_source_roots(run, &verification.input.ordered_results)
        {
            Ok(sources) => sources,
            Err(error) => return indeterminate_execution("source-roots", &error),
        };
        let (workspace_digest, bindings) =
            match self.materialize_work_verification(run, &verification.input, &sources) {
                Ok(materialized) => materialized,
                Err(error) => return indeterminate_execution("workspace-recovery", &error),
            };
        let serializable =
            verification.input.serializability.status == WorkSerializabilityStatus::Proven;
        let unknown_effects = verification.input.ordered_results.iter().any(|result| {
            run.attempts[&result.attempt_id]
                .result
                .as_ref()
                .is_some_and(|result| {
                    result.external_effects_status == ExternalEffectsStatus::Unknown
                })
        });
        let acceptance = evaluate_expected_files(&bindings, expected_files);
        let checks = vec![
            VerificationCheckEvidence {
                check_id: "immutable-replay".into(),
                command_digest: verification.input.validator_digest.clone(),
                output_digest: workspace_digest,
                outcome: VerificationCheckOutcome::Passed,
            },
            VerificationCheckEvidence {
                check_id: "serializability".into(),
                command_digest: digest_text("actual-effect-serializability-v1"),
                output_digest: verification.input.serializability.evidence_digest.clone(),
                outcome: if serializable {
                    VerificationCheckOutcome::Passed
                } else {
                    VerificationCheckOutcome::Indeterminate
                },
            },
            VerificationCheckEvidence {
                check_id: "external-effects".into(),
                command_digest: digest_text("external-effect-reconciliation-v1"),
                output_digest: digest_text(if unknown_effects {
                    "one or more external effects have no trusted receipt"
                } else {
                    "all selected results have host-confined or verified effects"
                }),
                outcome: if unknown_effects {
                    VerificationCheckOutcome::Indeterminate
                } else {
                    VerificationCheckOutcome::Passed
                },
            },
            VerificationCheckEvidence {
                check_id: "acceptance-profile".into(),
                command_digest: acceptance.command_digest,
                output_digest: acceptance.output_digest,
                outcome: if acceptance.passed {
                    VerificationCheckOutcome::Passed
                } else {
                    VerificationCheckOutcome::Failed
                },
            },
        ];
        if !serializable {
            WorkVerificationExecution {
                conclusion: VerificationConclusion::Indeterminate,
                checks,
                reason: format!(
                    "exact-file acceptance was evaluated, but serializability is not proven: {}",
                    verification.input.serializability.reason
                ),
            }
        } else if unknown_effects {
            WorkVerificationExecution {
                conclusion: VerificationConclusion::Indeterminate,
                checks,
                reason: "exact-file acceptance was evaluated, but external effects are not fully reconciled"
                    .into(),
            }
        } else if acceptance.passed {
            WorkVerificationExecution {
                conclusion: VerificationConclusion::Verified,
                checks,
                reason: "the independently materialized final root matched the exact-file acceptance profile"
                    .into(),
            }
        } else {
            WorkVerificationExecution {
                conclusion: VerificationConclusion::Rejected,
                checks,
                reason: acceptance.reason,
            }
        }
    }

    fn verification_source_roots(
        &self,
        run: &WorkRun,
        ordered_results: &[zeta_work_coordination::WorkResultRef],
    ) -> Result<BTreeMap<DirId, VerificationSourceRoot>, String> {
        let mut sources = BTreeMap::new();
        for result in ordered_results {
            let attempt = run
                .attempts
                .get(&result.attempt_id)
                .ok_or_else(|| format!("WorkAttempt {} does not exist", result.attempt_id))?;
            self.ensure_work_attempt_workspace(&run.work_run_id, attempt)?;
            let bindings = self
                .work_attempt_bindings
                .read()
                .map_err(|_| "WorkAttempt workspace binding lock poisoned".to_string())?
                .get(&(run.work_run_id.clone(), attempt.attempt_id.clone()))
                .cloned()
                .ok_or_else(|| {
                    "WorkAttempt workspace disappeared during verification".to_string()
                })?;
            for root in bindings.roots {
                insert_source_root(&mut sources, root)?;
            }
        }
        Ok(sources)
    }

    fn replay_verification_root(
        &self,
        source: &VerificationSourceRoot,
        records: &[TurnChangeSet],
    ) -> Result<VerificationRoot, String> {
        let checkpoint_digest =
            root_checkpoint_digest(&source.checkpoint).map_err(|error| error.to_string())?;
        let state = match &source.checkpoint.state {
            RootState::Git { repositories } => {
                let mut final_repositories = Vec::with_capacity(repositories.len());
                for checkpoint in repositories {
                    let binding = source
                        .managed
                        .repositories()
                        .iter()
                        .find(|repository| repository.repository_id() == checkpoint.repository_id)
                        .ok_or_else(|| {
                            format!(
                                "managed verification source omitted repository {}",
                                checkpoint.repository_id
                            )
                        })?;
                    let repository_root = binding.source_repository_root().to_path_buf();
                    let checkpoint = checkpoint.clone();
                    let applicable = records
                        .iter()
                        .filter(|record| {
                            record.repository_id == checkpoint.repository_id
                                && record.work_attempt.as_ref().is_some_and(|provenance| {
                                    provenance.source_root_dir_id == source.checkpoint.dir_id
                                })
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    let replayed = self.worktree_runtime.block_on(async {
                        let git = GitClient::system();
                        let repository = git
                            .open_repository(&repository_root)
                            .await
                            .map_err(|error| error.to_string())?;
                        let (target, target_tree) =
                            freeze_git_target(&git, &repository, &checkpoint.target).await?;
                        let mut final_tree = GitTreeId::new(target_tree.clone())
                            .map_err(|error| error.to_string())?;
                        for record in &applicable {
                            let before = GitTreeId::new(record.before_tree.clone())
                                .map_err(|error| error.to_string())?;
                            let after =
                                GitTreeId::new(record.after_tree.clone().ok_or_else(|| {
                                    "sealed ChangeSet omitted after tree".to_string()
                                })?)
                                .map_err(|error| error.to_string())?;
                            final_tree = match git
                                .replay_tree_delta(&repository, &before, &final_tree, &after)
                                .await
                                .map_err(|error| error.to_string())?
                            {
                                GitTreeReplayResult::Clean(tree) => tree,
                                GitTreeReplayResult::Conflict { paths } => {
                                    return Err(format!(
                                        "immutable replay conflicted in repository {} at {paths:?}",
                                        checkpoint.repository_id
                                    ));
                                }
                            };
                        }
                        Ok::<_, String>((target, target_tree, final_tree.as_str().to_string()))
                    })?;
                    final_repositories.push(GitVerificationRepository {
                        repository_id: checkpoint.repository_id,
                        relative_path: checkpoint.relative_path,
                        target: replayed.0,
                        target_tree: replayed.1,
                        final_tree: replayed.2,
                    });
                }
                VerificationRootState::Git {
                    repositories: final_repositories,
                }
            }
            RootState::Directory { .. } => {
                let store = source
                    .managed
                    .snapshot_store()
                    .ok_or_else(|| "managed directory omitted its snapshot store".to_string())?;
                let snapshots = DirectorySnapshotStore::new(store);
                let target_snapshot_id = snapshots.capture(source.source.canonical_path())?;
                let repository_id = source.managed.repositories()[0].repository_id();
                let mut final_snapshot_id = target_snapshot_id.clone();
                for record in records.iter().filter(|record| {
                    record.repository_id == repository_id
                        && record.work_attempt.as_ref().is_some_and(|provenance| {
                            provenance.source_root_dir_id == source.checkpoint.dir_id
                        })
                }) {
                    final_snapshot_id = match snapshots.replay(
                        &record.before_tree,
                        &final_snapshot_id,
                        record
                            .after_tree
                            .as_deref()
                            .ok_or_else(|| "sealed ChangeSet omitted after snapshot".to_string())?,
                    )? {
                        DirectoryReplayResult::Clean(snapshot) => snapshot,
                        DirectoryReplayResult::Conflict(paths) => {
                            return Err(format!(
                                "immutable directory replay conflicted at {paths:?}"
                            ));
                        }
                    };
                }
                VerificationRootState::Directory {
                    target_snapshot_id,
                    final_snapshot_id,
                }
            }
        };
        Ok(VerificationRoot {
            source_dir_id: source.checkpoint.dir_id.clone(),
            checkpoint_digest,
            state,
        })
    }

    fn ensure_work_verification_workspace(
        &self,
        run: &WorkRun,
        input: &WorkVerificationInput,
        sources: &BTreeMap<DirId, VerificationSourceRoot>,
    ) -> Result<ContentDigest, String> {
        self.materialize_work_verification(run, input, sources)
            .map(|(digest, _)| digest)
    }

    fn materialize_work_verification(
        &self,
        run: &WorkRun,
        input: &WorkVerificationInput,
        sources: &BTreeMap<DirId, VerificationSourceRoot>,
    ) -> Result<(ContentDigest, Vec<ManagedDirBinding>), String> {
        let key = verification_key(&run.work_run_id, input).map_err(|error| error.to_string())?;
        let mut manifests = Vec::with_capacity(input.roots.len());
        let mut bindings = Vec::with_capacity(input.roots.len());
        for root in &input.roots {
            let source = sources
                .get(&root.source_dir_id)
                .ok_or_else(|| "verification root has no exact source binding".to_string())?;
            let owner = ManagedDirOwner::VerificationRoot {
                work_run_id: run.work_run_id.to_string(),
                verification_key: key.to_string(),
                source_dir_id: root.source_dir_id.to_string(),
            };
            let request = verification_provision_request(root, source, owner)?;
            let binding = self
                .worktree_runtime
                .block_on(self.worktrees.provision(&request))
                .map_err(|error| error.to_string())?;
            validate_verification_binding(root, source, &binding)?;
            verify_materialized_final_state(&self.worktree_runtime, root, &binding)?;
            manifests.push(
                binding
                    .manifest_digest()
                    .map_err(|error| error.to_string())?,
            );
            bindings.push(binding);
        }
        let output = self
            .worktrees
            .provision_output(&ManagedOutputOwner::verification(
                run.work_run_id.to_string(),
                key.to_string(),
            ))
            .map_err(|error| error.to_string())?;
        let digest = canonical_digest(&(
            1_u32,
            &key,
            manifests,
            output.manifest_digest(),
            output.dir_id(),
        ))?;
        Ok((digest, bindings))
    }
}

#[cfg(feature = "multi-agent-evals")]
struct ExpectedFileEvaluation {
    passed: bool,
    command_digest: ContentDigest,
    output_digest: ContentDigest,
    reason: String,
}

#[cfg(feature = "multi-agent-evals")]
fn evaluate_expected_files(
    bindings: &[ManagedDirBinding],
    expected_files: &BTreeMap<String, Vec<u8>>,
) -> ExpectedFileEvaluation {
    let command_digest = canonical_digest(&(1_u32, "exact-files-v1", expected_files))
        .unwrap_or_else(|_| digest_text("exact-file-command-encoding-failed"));
    if bindings.len() != 1 || expected_files.is_empty() {
        return ExpectedFileEvaluation {
            passed: false,
            command_digest,
            output_digest: digest_text(
                "exact-file profile requires one root and one or more files",
            ),
            reason:
                "exact-file profile requires one materialized root and one or more expected files"
                    .into(),
        };
    }
    let mut actual = BTreeMap::new();
    let mut passed = true;
    for (path, expected) in expected_files {
        let relative = Path::new(path);
        let valid = !path.is_empty()
            && !relative.is_absolute()
            && relative
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)));
        let bytes = valid
            .then(|| std::fs::read(bindings[0].dir().join(relative)).ok())
            .flatten();
        passed &= bytes.as_deref() == Some(expected.as_slice());
        actual.insert(
            path.clone(),
            bytes.map(|bytes| ContentDigest::sha256(&bytes)),
        );
    }
    ExpectedFileEvaluation {
        passed,
        command_digest,
        output_digest: canonical_digest(&(1_u32, actual))
            .unwrap_or_else(|_| digest_text("exact-file-output-encoding-failed")),
        reason: if passed {
            "the exact expected files matched".into()
        } else {
            "one or more independently materialized files were missing or had different content"
                .into()
        },
    }
}

fn insert_source_root(
    sources: &mut BTreeMap<DirId, VerificationSourceRoot>,
    root: WorkAttemptRootBinding,
) -> Result<(), String> {
    let candidate = VerificationSourceRoot {
        checkpoint: root.checkpoint,
        source: root.source,
        managed: root.managed,
    };
    if let Some(existing) = sources.get(&candidate.checkpoint.dir_id) {
        if existing.checkpoint != candidate.checkpoint || existing.source != candidate.source {
            return Err("selected WorkAttempts disagree on one verification root".into());
        }
        return Ok(());
    }
    sources.insert(candidate.checkpoint.dir_id.clone(), candidate);
    Ok(())
}

fn validate_verification_change_set(
    run: &WorkRun,
    attempt: &zeta_work_coordination::WorkAttempt,
    record: &TurnChangeSet,
) -> Result<(), String> {
    let provenance = record
        .work_attempt
        .as_ref()
        .ok_or_else(|| "verification ChangeSet omitted WorkAttempt provenance".to_string())?;
    if provenance.work_run_id != run.work_run_id
        || provenance.attempt_id != attempt.attempt_id
        || attempt.execution_id.as_ref() != Some(&provenance.execution_id)
        || provenance.contract_id != attempt.contract.contract_id
        || provenance.contract_revision != attempt.contract.revision
        || record.session_id != attempt.session_id
        || record.thread_id != attempt.thread_id
        || record.capture_state != CaptureState::Sealed
        || record.terminal_state != Some(TerminalTurnState::Completed)
        || record.after_tree.is_none()
        || record.attribution_incomplete
        || record.commit_state != CommitState::Idle
    {
        return Err("verification ChangeSet provenance or terminal state is inconsistent".into());
    }
    let checkpoint = attempt
        .roots
        .iter()
        .find(|root| root.dir_id == provenance.source_root_dir_id)
        .ok_or_else(|| "verification ChangeSet names an unknown source root".to_string())?;
    if root_checkpoint_digest(checkpoint).map_err(|error| error.to_string())?
        != provenance.root_checkpoint_digest
        || matches!(checkpoint.state, RootState::Directory { .. })
            != matches!(record.snapshot_backend, SnapshotBackend::Directory { .. })
    {
        return Err("verification ChangeSet root checkpoint is inconsistent".into());
    }
    Ok(())
}

fn validate_change_set_order(records: &[TurnChangeSet]) -> Result<(), String> {
    let positions = records
        .iter()
        .enumerate()
        .map(|(index, record)| (record.change_set_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    if positions.len() != records.len() {
        return Err("verification repeats a ChangeSet identity".into());
    }
    for (index, record) in records.iter().enumerate() {
        if record.dependencies.iter().any(|dependency| {
            positions
                .get(dependency)
                .is_none_or(|dependency_index| *dependency_index >= index)
        }) {
            return Err("verification ChangeSet order has an unresolved dependency".into());
        }
    }
    Ok(())
}

async fn freeze_git_target(
    git: &GitClient,
    repository: &zeta_git::GitRepository,
    target: &GitRootTarget,
) -> Result<(GitRootTarget, String), String> {
    match target {
        GitRootTarget::Branch { name, .. } => {
            let branch = git
                .local_branches(repository)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .find(|branch| branch.name() == name)
                .ok_or_else(|| format!("target branch {name} was deleted"))?;
            let object_id = branch.object_id().to_string();
            let tree = git
                .resolve_tree(repository, &object_id)
                .await
                .map_err(|error| error.to_string())?;
            Ok((
                GitRootTarget::Branch {
                    name: name.clone(),
                    expected_head: object_id,
                },
                tree.as_str().to_string(),
            ))
        }
        GitRootTarget::UnbornBranch {
            name,
            anchor_object_id,
        } => {
            let branch = git
                .local_branches(repository)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .find(|branch| branch.name() == name);
            if let Some(branch) = branch {
                let object_id = branch.object_id().to_string();
                let tree = git
                    .resolve_tree(repository, &object_id)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok((
                    GitRootTarget::Branch {
                        name: name.clone(),
                        expected_head: object_id,
                    },
                    tree.as_str().to_string(),
                ))
            } else {
                let tree = git
                    .empty_tree(repository)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok((
                    GitRootTarget::UnbornBranch {
                        name: name.clone(),
                        anchor_object_id: anchor_object_id.clone(),
                    },
                    tree.as_str().to_string(),
                ))
            }
        }
        GitRootTarget::Detached { object_id } => {
            let tree = git
                .resolve_tree(repository, object_id)
                .await
                .map_err(|error| error.to_string())?;
            Ok((target.clone(), tree.as_str().to_string()))
        }
    }
}

fn verification_provision_request(
    root: &VerificationRoot,
    source: &VerificationSourceRoot,
    owner: ManagedDirOwner,
) -> Result<ManagedDirProvisionRequest, String> {
    let (managed_source, target, repository_targets) = match &root.state {
        VerificationRootState::Git { repositories } => {
            let primary = repositories
                .iter()
                .find(|repository| Path::new(&repository.relative_path) == Path::new("."))
                .ok_or_else(|| {
                    "verification Git root omitted its primary repository".to_string()
                })?;
            let trees = repositories
                .iter()
                .map(|repository| {
                    (
                        PathBuf::from(&repository.relative_path),
                        repository.final_tree.clone(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let targets = repositories
                .iter()
                .filter(|repository| repository.relative_path != ".")
                .map(|repository| {
                    (
                        PathBuf::from(&repository.relative_path),
                        managed_target(&repository.target),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            (
                ManagedDirSource::ImmutableTree {
                    source_directory: source.source.canonical_path().to_path_buf(),
                    tree_id: primary.final_tree.clone(),
                    repository_trees: trees,
                },
                managed_target(&primary.target),
                targets,
            )
        }
        VerificationRootState::Directory {
            final_snapshot_id, ..
        } => (
            ManagedDirSource::ImmutableTree {
                source_directory: source.source.canonical_path().to_path_buf(),
                tree_id: final_snapshot_id.clone(),
                repository_trees: BTreeMap::new(),
            },
            ManagedDirTarget::SourceHead,
            BTreeMap::new(),
        ),
    };
    Ok(ManagedDirProvisionRequest {
        source: managed_source,
        target,
        repository_targets,
        source_dir_id: root.source_dir_id.to_string(),
        owner,
    })
}

fn managed_target(target: &GitRootTarget) -> ManagedDirTarget {
    match target {
        GitRootTarget::Branch {
            name,
            expected_head,
        } => ManagedDirTarget::Branch {
            name: name.clone(),
            object_id: expected_head.clone(),
        },
        GitRootTarget::UnbornBranch {
            name,
            anchor_object_id,
        } => ManagedDirTarget::UnbornBranch {
            name: name.clone(),
            anchor_object_id: anchor_object_id.clone(),
        },
        GitRootTarget::Detached { object_id } => ManagedDirTarget::Detached {
            object_id: object_id.clone(),
        },
    }
}

fn validate_verification_binding(
    root: &VerificationRoot,
    source: &VerificationSourceRoot,
    binding: &ManagedDirBinding,
) -> Result<(), String> {
    if source.source.id() != root.source_dir_id
        || binding.source_dir_id() != root.source_dir_id.as_str()
    {
        return Err("verification workspace source identity is inconsistent".into());
    }
    match &root.state {
        VerificationRootState::Directory {
            final_snapshot_id, ..
        } => {
            if binding.kind() != ManagedDirKind::Directory
                || binding.baseline_tree() != final_snapshot_id
                || binding.repositories().len() != 1
            {
                return Err("verification directory binding has the wrong final snapshot".into());
            }
        }
        VerificationRootState::Git { repositories } => {
            if binding.kind() != ManagedDirKind::Git
                || binding.repositories().len() != repositories.len()
            {
                return Err("verification Git binding has the wrong repository set".into());
            }
            for repository in repositories {
                let actual = binding
                    .repositories()
                    .iter()
                    .find(|actual| actual.repository_id() == repository.repository_id)
                    .ok_or_else(|| "verification binding omitted a repository".to_string())?;
                if actual.relative_path() != Path::new(&repository.relative_path)
                    || actual.baseline_tree() != repository.final_tree
                    || !managed_target_matches(&repository.target, actual)
                {
                    return Err("verification repository binding is inconsistent".into());
                }
            }
        }
    }
    Ok(())
}

fn managed_target_matches(
    target: &GitRootTarget,
    binding: &zeta_worktree::ManagedRepositoryBinding,
) -> bool {
    match target {
        GitRootTarget::Branch {
            name,
            expected_head,
        } => {
            binding.target_branch() == Some(name)
                && !binding.target_unborn()
                && binding.target_head() == expected_head
        }
        GitRootTarget::UnbornBranch {
            name,
            anchor_object_id,
        } => {
            binding.target_branch() == Some(name)
                && binding.target_unborn()
                && binding.target_head() == anchor_object_id
        }
        GitRootTarget::Detached { object_id } => {
            binding.target_branch().is_none()
                && !binding.target_unborn()
                && binding.target_head() == object_id
        }
    }
}

fn verify_materialized_final_state(
    runtime: &tokio::runtime::Runtime,
    root: &VerificationRoot,
    binding: &ManagedDirBinding,
) -> Result<(), String> {
    match &root.state {
        VerificationRootState::Directory {
            final_snapshot_id, ..
        } => {
            let actual = DirectorySnapshotStore::new(
                binding
                    .snapshot_store()
                    .ok_or_else(|| "verification directory omitted snapshot store".to_string())?,
            )
            .capture(binding.dir())?;
            if &actual != final_snapshot_id {
                return Err(
                    "materialized verification directory differs from its final snapshot".into(),
                );
            }
        }
        VerificationRootState::Git { repositories } => {
            for expected in repositories {
                let actual = binding
                    .repositories()
                    .iter()
                    .find(|actual| actual.repository_id() == expected.repository_id)
                    .ok_or_else(|| {
                        "materialized verification root omitted repository".to_string()
                    })?;
                let tree = runtime.block_on(async {
                    let git = GitClient::system();
                    let repository = git
                        .open_repository(actual.worktree_root())
                        .await
                        .map_err(|error| error.to_string())?;
                    git.capture_worktree_tree(&repository)
                        .await
                        .map_err(|error| error.to_string())
                })?;
                if tree.as_str() != expected.final_tree {
                    return Err(
                        "materialized verification Git tree differs from its final tree".into(),
                    );
                }
            }
        }
    }
    Ok(())
}

fn indeterminate_execution(check_id: &str, reason: &str) -> WorkVerificationExecution {
    WorkVerificationExecution {
        conclusion: VerificationConclusion::Indeterminate,
        checks: vec![VerificationCheckEvidence {
            check_id: check_id.into(),
            command_digest: digest_text(VERIFIER_REVISION),
            output_digest: digest_text(reason),
            outcome: VerificationCheckOutcome::Indeterminate,
        }],
        reason: reason.into(),
    }
}

fn digest_set(values: &BTreeSet<ContentDigest>) -> ContentDigest {
    canonical_digest(&(1_u32, values)).unwrap_or_else(|_| digest_text("digest-encoding-failed"))
}

fn digest_text(value: &str) -> ContentDigest {
    ContentDigest::sha256(value.as_bytes())
}

fn canonical_digest(value: &impl Serialize) -> Result<ContentDigest, String> {
    serde_json::to_vec(value)
        .map(|encoded| ContentDigest::sha256(&encoded))
        .map_err(|error| error.to_string())
}
