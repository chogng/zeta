use super::turn_changes_runtime::TurnChangesRuntime;
use super::work_attempt_effects::work_attempt_effects;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use zeta_protocol::ContentDigest;
use zeta_protocol::TurnStatus;
use zeta_protocol::WorkAttemptId;
use zeta_turn_changes::CaptureState;
use zeta_turn_changes::CommitState;
use zeta_turn_changes::SnapshotBackend;
use zeta_turn_changes::TerminalTurnState;
use zeta_turn_changes::TurnChangeSet;
use zeta_turn_changes::TurnChangeStore;
use zeta_work_coordination::RootState;
use zeta_work_coordination::WorkAttemptChangeEvidenceRef;
#[cfg(feature = "multi-agent-evals")]
use zeta_work_coordination::WorkAttemptResult;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::work_attempt_result_digest;
use zeta_worktree::ManagedDirKind;

impl TurnChangesRuntime {
    /// Derives a sealing claim from the current durable Thread and managed workspace facts.
    ///
    /// The returned value is still revalidated by [`Self::validate_attempt_result`] when the
    /// `SealAttempt` command is applied. This helper gives trusted hosts a canonical way to request
    /// sealing without accepting result identities invented by an Agent.
    #[cfg(feature = "multi-agent-evals")]
    pub(super) fn derive_attempt_result(
        &self,
        run: &WorkRun,
        attempt_id: &WorkAttemptId,
    ) -> Result<WorkAttemptResult, String> {
        let attempt = run
            .attempts
            .get(attempt_id)
            .ok_or_else(|| format!("WorkAttempt {attempt_id} does not exist"))?;
        let execution_id = attempt
            .execution_id
            .as_ref()
            .ok_or_else(|| "WorkAttempt result omitted its execution identity".to_string())?;
        self.ensure_work_attempt_workspace(&run.work_run_id, attempt)?;
        let bindings = self
            .work_attempt_bindings
            .read()
            .map_err(|_| "WorkAttempt workspace binding lock poisoned".to_string())?
            .get(&(run.work_run_id.clone(), attempt.attempt_id.clone()))
            .cloned()
            .ok_or_else(|| "WorkAttempt workspace evidence is unavailable".to_string())?;
        let records = self
            .store
            .list_for_thread(&attempt.thread_id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|record| {
                record.work_attempt.as_ref().is_some_and(|provenance| {
                    provenance.work_run_id == run.work_run_id
                        && provenance.attempt_id == attempt.attempt_id
                        && provenance.execution_id == *execution_id
                        && provenance.contract_id == attempt.contract.contract_id
                        && provenance.contract_revision == attempt.contract.revision
                })
            })
            .collect::<Vec<_>>();
        if records.is_empty() {
            return Err("WorkAttempt result has no captured Turn evidence".into());
        }
        if let Some(record) = records
            .iter()
            .find(|record| record.capture_state != CaptureState::Sealed)
        {
            let capture_failure = self
                .capture_failures
                .read()
                .map_err(|_| "Turn capture failure lock poisoned".to_string())?
                .get(&record.turn_id)
                .cloned();
            return Err(format!(
                "WorkAttempt ChangeSet {} remained {:?}; capture failure: {:?}",
                record.change_set_id, record.capture_state, capture_failure
            ));
        }
        let evidence = records
            .iter()
            .map(|record| {
                record
                    .evidence_digest()
                    .map(|evidence_digest| WorkAttemptChangeEvidenceRef {
                        change_set_id: record.change_set_id.clone(),
                        evidence_digest,
                    })
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let change_set_ids = records
            .iter()
            .map(|record| record.change_set_id.clone())
            .collect::<Vec<_>>();
        let turn_ids = records
            .iter()
            .map(|record| record.turn_id.clone())
            .collect::<BTreeSet<_>>();
        let effects = work_attempt_effects(
            &self
                .threads
                .read_thread(&attempt.thread_id)
                .map_err(|error| error.to_string())?,
            &turn_ids,
        )?;
        let private_output_digest = self
            .worktrees
            .capture_output(&bindings.output)
            .map_err(|error| error.to_string())?;
        let result_digest = work_attempt_result_digest(
            &run.work_run_id,
            run.topology_revision,
            attempt,
            &evidence,
            &private_output_digest,
            &effects.digest,
            effects.status,
        )
        .map_err(|error| error.to_string())?;
        Ok(WorkAttemptResult {
            result_digest,
            change_set_ids,
            private_output_digest,
            external_effects_digest: effects.digest,
            external_effects_status: effects.status,
        })
    }

    pub(super) fn validate_attempt_result(
        &self,
        run: &WorkRun,
        attempt_id: &WorkAttemptId,
        change_set_ids: &[zeta_turn_changes::ChangeSetId],
        claimed_private_output_digest: &ContentDigest,
        external_effects_digest: &ContentDigest,
        external_effects_status: zeta_work_coordination::ExternalEffectsStatus,
        claimed_result_digest: &ContentDigest,
    ) -> Result<(), String> {
        let thread_id = run
            .attempts
            .get(attempt_id)
            .ok_or_else(|| format!("WorkAttempt {attempt_id} does not exist"))?
            .thread_id
            .clone();
        if !self
            .sealing_threads
            .write()
            .map_err(|_| "WorkAttempt sealing lock poisoned".to_string())?
            .insert(thread_id.clone())
        {
            return Err("WorkAttempt result sealing is already in progress".into());
        }
        let result = self.validate_attempt_result_inside(
            run,
            attempt_id,
            change_set_ids,
            claimed_private_output_digest,
            external_effects_digest,
            external_effects_status,
            claimed_result_digest,
        );
        if result.is_err() {
            self.release_attempt_result_barrier(&thread_id);
        }
        result
    }

    pub(super) fn release_attempt_result_barrier(&self, thread_id: &zeta_protocol::ThreadId) {
        self.sealing_threads
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(thread_id);
    }

    fn validate_attempt_result_inside(
        &self,
        run: &WorkRun,
        attempt_id: &WorkAttemptId,
        change_set_ids: &[zeta_turn_changes::ChangeSetId],
        claimed_private_output_digest: &ContentDigest,
        external_effects_digest: &ContentDigest,
        external_effects_status: zeta_work_coordination::ExternalEffectsStatus,
        claimed_result_digest: &ContentDigest,
    ) -> Result<(), String> {
        let attempt = run
            .attempts
            .get(attempt_id)
            .ok_or_else(|| format!("WorkAttempt {attempt_id} does not exist"))?;
        let execution_id = attempt
            .execution_id
            .as_ref()
            .ok_or_else(|| "WorkAttempt result omitted its execution identity".to_string())?;
        let contract = run
            .contract(&attempt.contract.contract_id, attempt.contract.revision)
            .ok_or_else(|| "WorkAttempt result references an unknown contract".to_string())?;
        if contract.topology_revision != run.topology_revision {
            return Err("WorkAttempt collaboration topology is stale".into());
        }
        let thread = self
            .threads
            .read_thread(&attempt.thread_id)
            .map_err(|error| error.to_string())?;
        if thread.session_id != attempt.session_id
            || thread.turns.iter().any(|turn| {
                matches!(
                    turn.status,
                    TurnStatus::Created
                        | TurnStatus::Running
                        | TurnStatus::WaitingForApproval
                        | TurnStatus::WaitingForUserInput
                        | TurnStatus::WaitingForCapability
                        | TurnStatus::Cancelling
                )
            })
        {
            return Err("WorkAttempt result can be sealed only at a safe Thread boundary".into());
        }
        self.ensure_work_attempt_workspace(&run.work_run_id, attempt)?;
        let bindings = self
            .work_attempt_bindings
            .read()
            .map_err(|_| "WorkAttempt workspace binding lock poisoned".to_string())?
            .get(&(run.work_run_id.clone(), attempt.attempt_id.clone()))
            .cloned()
            .ok_or_else(|| "WorkAttempt workspace evidence is unavailable".to_string())?;

        let records = self
            .store
            .list_for_thread(&attempt.thread_id)
            .map_err(|error| error.to_string())?;
        let mut selected = Vec::new();
        for record in records {
            let Some(provenance) = &record.work_attempt else {
                continue;
            };
            if provenance.work_run_id != run.work_run_id
                || provenance.attempt_id != attempt.attempt_id
            {
                continue;
            }
            if provenance.execution_id != *execution_id
                || provenance.contract_id != attempt.contract.contract_id
                || provenance.contract_revision != attempt.contract.revision
                || record.session_id != attempt.session_id
                || record.thread_id != attempt.thread_id
            {
                return Err("ChangeSet provenance does not match the exact WorkAttempt".into());
            }
            selected.push(record);
        }
        if selected.is_empty() {
            return Err("WorkAttempt result has no captured Turn evidence".into());
        }
        if selected
            .iter()
            .map(|record| &record.change_set_id)
            .ne(change_set_ids.iter())
        {
            return Err(
                "WorkAttempt result must include every ChangeSet in durable capture order".into(),
            );
        }

        let expected_roots = attempt
            .roots
            .iter()
            .map(|root| {
                zeta_work_coordination::root_checkpoint_digest(root)
                    .map(|digest| (root.dir_id.clone(), digest))
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let expected_directory_backends = attempt
            .roots
            .iter()
            .map(|root| {
                (
                    root.dir_id.clone(),
                    matches!(root.state, RootState::Directory { .. }),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let expected_repositories = bindings
            .roots
            .iter()
            .flat_map(|root| {
                root.managed.repositories().iter().map(|repository| {
                    Ok((
                        repository.repository_id().to_string(),
                        (
                            root.checkpoint.dir_id.clone(),
                            zeta_file_access::Dir::open_local(root.managed.dir())
                                .map_err(|error| error.to_string())?
                                .id(),
                        ),
                    ))
                })
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        let actual_repositories = selected
            .iter()
            .map(|record| record.repository_id.clone())
            .collect::<BTreeSet<_>>();
        if actual_repositories != expected_repositories.keys().cloned().collect() {
            return Err("WorkAttempt ChangeSets do not cover every managed repository".into());
        }

        let positions = selected
            .iter()
            .enumerate()
            .map(|(index, record)| (record.change_set_id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut evidence = Vec::with_capacity(selected.len());
        for (index, record) in selected.iter().enumerate() {
            validate_change_set(
                record,
                index,
                &positions,
                &expected_roots,
                &expected_repositories,
                &expected_directory_backends,
            )?;
            evidence.push(WorkAttemptChangeEvidenceRef {
                change_set_id: record.change_set_id.clone(),
                evidence_digest: record
                    .evidence_digest()
                    .map_err(|error| error.to_string())?,
            });
        }
        let turn_ids = selected
            .iter()
            .map(|record| record.turn_id.clone())
            .collect::<BTreeSet<_>>();
        let effects = work_attempt_effects(&thread, &turn_ids)?;
        if effects.digest != *external_effects_digest || effects.status != external_effects_status {
            return Err(
                "WorkAttempt external effect claim does not match the authoritative Tool log"
                    .into(),
            );
        }
        self.validate_final_root_states(&bindings, &selected)?;
        let private_output_digest = self
            .worktrees
            .capture_output(&bindings.output)
            .map_err(|error| error.to_string())?;
        if &private_output_digest != claimed_private_output_digest {
            return Err("WorkAttempt private output digest does not match its contents".into());
        }
        let result_digest = work_attempt_result_digest(
            &run.work_run_id,
            run.topology_revision,
            attempt,
            &evidence,
            &private_output_digest,
            external_effects_digest,
            external_effects_status,
        )
        .map_err(|error| error.to_string())?;
        if &result_digest != claimed_result_digest {
            return Err("WorkAttempt result digest does not match its evidence".into());
        }
        Ok(())
    }

    fn validate_final_root_states(
        &self,
        bindings: &super::work_attempt_workspace::WorkAttemptWorkspaceBindings,
        records: &[TurnChangeSet],
    ) -> Result<(), String> {
        for root in &bindings.roots {
            for repository in root.managed.repositories() {
                let expected = records
                    .iter()
                    .rev()
                    .find(|record| record.repository_id == repository.repository_id())
                    .and_then(|record| record.after_tree.as_deref())
                    .ok_or_else(|| {
                        format!(
                            "WorkAttempt result omitted final state for repository {}",
                            repository.repository_id()
                        )
                    })?;
                let actual = match root.managed.kind() {
                    ManagedDirKind::Git => self.worktree_runtime.block_on(async {
                        let git = zeta_git::GitClient::system();
                        let repository = git
                            .open_repository(repository.worktree_root())
                            .await
                            .map_err(|error| error.to_string())?;
                        git.capture_worktree_tree(&repository)
                            .await
                            .map(|tree| tree.as_str().to_string())
                            .map_err(|error| error.to_string())
                    })?,
                    ManagedDirKind::Directory => {
                        let store = root.managed.snapshot_store().ok_or_else(|| {
                            "managed directory omitted its snapshot store".to_string()
                        })?;
                        zeta_turn_changes::DirectorySnapshotStore::new(store)
                            .capture(repository.worktree_root())?
                    }
                };
                if actual != expected {
                    return Err(format!(
                        "managed repository {} changed after its last sealed ChangeSet",
                        repository.repository_id()
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_change_set(
    record: &TurnChangeSet,
    index: usize,
    positions: &BTreeMap<zeta_turn_changes::ChangeSetId, usize>,
    expected_roots: &BTreeMap<zeta_file_access::DirId, ContentDigest>,
    expected_repositories: &BTreeMap<String, (zeta_file_access::DirId, zeta_file_access::DirId)>,
    expected_directory_backends: &BTreeMap<zeta_file_access::DirId, bool>,
) -> Result<(), String> {
    let provenance = record
        .work_attempt
        .as_ref()
        .ok_or_else(|| "WorkAttempt ChangeSet omitted provenance".to_string())?;
    let (source_dir_id, managed_dir_id) = expected_repositories
        .get(&record.repository_id)
        .ok_or_else(|| "WorkAttempt ChangeSet names an unexpected repository".to_string())?;
    if &provenance.source_root_dir_id != source_dir_id
        || &provenance.managed_root_dir_id != managed_dir_id
        || expected_roots.get(source_dir_id) != Some(&provenance.root_checkpoint_digest)
        || record.capture_state != CaptureState::Sealed
        || record.terminal_state != Some(TerminalTurnState::Completed)
        || record.after_tree.is_none()
        || record.attribution_incomplete
        || record.commit_state != CommitState::Idle
    {
        return Err("WorkAttempt ChangeSet is incomplete, ambiguous, or already published".into());
    }
    for dependency in &record.dependencies {
        if positions
            .get(dependency)
            .is_none_or(|dependency_index| *dependency_index >= index)
        {
            return Err("WorkAttempt ChangeSet has an unresolved or non-serial dependency".into());
        }
    }
    if expected_directory_backends.get(source_dir_id).copied()
        != Some(matches!(
            record.snapshot_backend,
            SnapshotBackend::Directory { .. }
        ))
    {
        return Err("WorkAttempt ChangeSet snapshot backend is inconsistent".into());
    }
    Ok(())
}
