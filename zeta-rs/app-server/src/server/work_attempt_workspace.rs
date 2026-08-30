use super::turn_changes_runtime::TurnChangesRuntime;
use crate::dir_grants::WorkAttemptDirIdentity;
use crate::dir_grants::WorkAttemptDirRoot;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use zeta_file_access::Dir;
use zeta_file_access::DirId;
use zeta_file_access::Permission;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkRunId;
use zeta_turn_changes::WorkAttemptChangeProvenance;
use zeta_work_coordination::GitRepositoryCheckpoint;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::ManagedRootBinding as CoordinationRootBinding;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::WorkAttempt;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkAttemptWorkspace;
use zeta_work_coordination::root_checkpoint_digest;
use zeta_worktree::ManagedDirBinding;
use zeta_worktree::ManagedDirKind;
use zeta_worktree::ManagedDirOwner;
use zeta_worktree::ManagedDirProvisionRequest;
use zeta_worktree::ManagedDirSource;
use zeta_worktree::ManagedDirTarget;
use zeta_worktree::ManagedOutputBinding;
use zeta_worktree::ManagedOutputOwner;

#[derive(Clone)]
pub(super) struct WorkAttemptRootBinding {
    pub(super) checkpoint: RootCheckpoint,
    pub(super) source: Dir,
    pub(super) managed: ManagedDirBinding,
}

#[derive(Clone)]
pub(super) struct WorkAttemptWorkspaceBindings {
    work_run_id: WorkRunId,
    attempt_id: WorkAttemptId,
    thread_id: zeta_protocol::ThreadId,
    contract_id: WorkContractId,
    contract_revision: u64,
    primary_root_dir_id: DirId,
    pub(super) roots: Vec<WorkAttemptRootBinding>,
    pub(super) output: ManagedOutputBinding,
}

#[derive(Clone)]
pub(super) struct ActiveWorkAttemptWorkspace {
    identity: WorkAttemptDirIdentity,
    bindings: WorkAttemptWorkspaceBindings,
}

#[derive(Clone)]
pub(super) struct ExecutionRootBinding {
    pub(super) binding: ManagedDirBinding,
    pub(super) source: Dir,
    pub(super) work_attempt: Option<WorkAttemptChangeProvenance>,
    pub(super) primary: bool,
}

impl TurnChangesRuntime {
    pub(super) fn ensure_work_attempt_workspace(
        &self,
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
    ) -> Result<(Vec<CoordinationRootBinding>, DirId), String> {
        let _guard = self
            .workspace_gate
            .lock()
            .map_err(|_| "WorkAttempt workspace lock poisoned".to_string())?;
        let key = (work_run_id.clone(), attempt.attempt_id.clone());
        if let Some(bindings) = self
            .work_attempt_bindings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .cloned()
        {
            bindings.validate_attempt(work_run_id, attempt)?;
            return bindings.coordination_bindings();
        }
        let bindings = match &attempt.workspace {
            WorkAttemptWorkspace::Provisioning => {
                self.provision_work_attempt_bindings(work_run_id, attempt)?
            }
            WorkAttemptWorkspace::Ready {
                roots,
                private_output_dir_id,
            } => self.recover_work_attempt_bindings(
                work_run_id,
                attempt,
                roots,
                private_output_dir_id,
            )?,
            WorkAttemptWorkspace::Failed { reason } => {
                return Err(format!("WorkAttempt workspace failed: {reason}"));
            }
        };
        let result = bindings.coordination_bindings()?;
        self.work_attempt_bindings
            .write()
            .map_err(|_| "WorkAttempt workspace binding lock poisoned".to_string())?
            .insert(key, bindings);
        Ok(result)
    }

    pub(super) fn activate_work_attempt_workspace(
        &self,
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
    ) -> Result<(), String> {
        if self
            .sealing_threads
            .read()
            .map_err(|_| "WorkAttempt sealing lock poisoned".to_string())?
            .contains(&attempt.thread_id)
        {
            return Err("WorkAttempt result sealing prevents execution activation".into());
        }
        if !matches!(
            attempt.execution_status,
            WorkAttemptExecutionStatus::Exploring | WorkAttemptExecutionStatus::Writing
        ) {
            return Err("only an active WorkAttempt may own a Thread directory scope".into());
        }
        let execution_id = attempt
            .execution_id
            .clone()
            .ok_or_else(|| "active WorkAttempt omitted its execution identity".to_string())?;
        self.ensure_work_attempt_workspace(work_run_id, attempt)?;
        let key = (work_run_id.clone(), attempt.attempt_id.clone());
        let bindings = self
            .work_attempt_bindings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .cloned()
            .ok_or_else(|| "WorkAttempt workspace disappeared after recovery".to_string())?;
        let identity = WorkAttemptDirIdentity {
            work_run_id: work_run_id.clone(),
            attempt_id: attempt.attempt_id.clone(),
            execution_id,
        };
        if let Some(active) = self
            .active_work_attempts
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&attempt.thread_id)
        {
            if active.identity == identity {
                return Ok(());
            }
            return Err("Thread is already bound to another WorkAttempt execution".into());
        }
        self.file_access.bind_work_attempt_dirs(
            attempt.thread_id.clone(),
            identity.clone(),
            attempt.primary_root_dir_id.clone(),
            bindings
                .roots
                .iter()
                .map(|root| {
                    Ok(WorkAttemptDirRoot {
                        source: root.source.clone(),
                        managed: Dir::open_local(root.managed.dir())
                            .map_err(|error| error.to_string())?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            Dir::open_local(bindings.output.root()).map_err(|error| error.to_string())?,
            Dir::open_local(&self.worktrees.settings().root).map_err(|error| error.to_string())?,
        )?;
        let primary = bindings.primary_root()?;
        if let Err(error) = self.hooks.bind_thread_dir(
            attempt.thread_id.clone(),
            Dir::open_local(primary.managed.dir()).map_err(|error| error.to_string())?,
        ) {
            let _ = self
                .file_access
                .unbind_work_attempt_dirs(&attempt.thread_id, &identity);
            return Err(error.to_string());
        }
        self.stop_watcher(&attempt.thread_id);
        if let Err(error) = self.start_watcher(
            attempt.thread_id.clone(),
            bindings
                .roots
                .iter()
                .map(|root| root.managed.dir().to_path_buf()),
        ) {
            let _ = self
                .file_access
                .unbind_work_attempt_dirs(&attempt.thread_id, &identity);
            self.restore_default_thread_services(&attempt.thread_id);
            return Err(error);
        }
        self.active_work_attempts
            .write()
            .map_err(|_| "active WorkAttempt lock poisoned".to_string())?
            .insert(
                attempt.thread_id.clone(),
                ActiveWorkAttemptWorkspace { identity, bindings },
            );
        Ok(())
    }

    pub(super) fn deactivate_work_attempt_workspace(
        &self,
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
    ) -> Result<(), String> {
        let active = self
            .active_work_attempts
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&attempt.thread_id)
            .cloned();
        let Some(active) = active else {
            self.release_attempt_result_barrier(&attempt.thread_id);
            return Ok(());
        };
        if &active.identity.work_run_id != work_run_id
            || active.identity.attempt_id != attempt.attempt_id
        {
            return Ok(());
        }
        self.file_access
            .unbind_work_attempt_dirs(&attempt.thread_id, &active.identity)?;
        self.active_work_attempts
            .write()
            .map_err(|_| "active WorkAttempt lock poisoned".to_string())?
            .remove(&attempt.thread_id);
        self.stop_watcher(&attempt.thread_id);
        self.restore_default_thread_services(&attempt.thread_id);
        self.release_attempt_result_barrier(&attempt.thread_id);
        Ok(())
    }

    pub(super) fn execution_roots(
        &self,
        thread_id: &zeta_protocol::ThreadId,
    ) -> Result<Vec<ExecutionRootBinding>, String> {
        if let Some(active) = self
            .active_work_attempts
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(thread_id)
            .cloned()
        {
            return active
                .bindings
                .roots
                .iter()
                .map(|root| {
                    Ok(ExecutionRootBinding {
                        binding: root.managed.clone(),
                        source: root.source.clone(),
                        primary: root.checkpoint.dir_id == active.bindings.primary_root_dir_id,
                        work_attempt: Some(WorkAttemptChangeProvenance {
                            work_run_id: active.bindings.work_run_id.clone(),
                            attempt_id: active.bindings.attempt_id.clone(),
                            execution_id: active.identity.execution_id.clone(),
                            contract_id: active.bindings.contract_id.clone(),
                            contract_revision: active.bindings.contract_revision,
                            source_root_dir_id: root.checkpoint.dir_id.clone(),
                            managed_root_dir_id: Dir::open_local(root.managed.dir())
                                .map_err(|error| error.to_string())?
                                .id(),
                            root_checkpoint_digest: root_checkpoint_digest(&root.checkpoint)
                                .map_err(|error| error.to_string())?,
                        }),
                    })
                })
                .collect();
        }
        self.binding(thread_id)
            .into_iter()
            .map(|binding| {
                Ok(ExecutionRootBinding {
                    source: Dir::open_local(binding.source_directory())
                        .map_err(|error| error.to_string())?,
                    binding,
                    work_attempt: None,
                    primary: true,
                })
            })
            .collect()
    }

    pub(super) fn execution_output(&self, thread_id: &zeta_protocol::ThreadId) -> Option<PathBuf> {
        self.active_work_attempts
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(thread_id)
            .map(|active| active.bindings.output.root().to_path_buf())
    }

    fn provision_work_attempt_bindings(
        &self,
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
    ) -> Result<WorkAttemptWorkspaceBindings, String> {
        let mut roots = Vec::with_capacity(attempt.roots.len());
        for checkpoint in &attempt.roots {
            let source = self.source_root(attempt, checkpoint)?;
            let owner = root_owner(work_run_id, attempt, checkpoint);
            let request = provision_request(checkpoint, &source, owner)?;
            let managed = self
                .worktree_runtime
                .block_on(self.worktrees.provision(&request))
                .map_err(|error| error.to_string())?;
            validate_managed_root(checkpoint, &source, &managed)?;
            roots.push(WorkAttemptRootBinding {
                checkpoint: checkpoint.clone(),
                source,
                managed,
            });
        }
        let output = self
            .worktrees
            .provision_output(&output_owner(work_run_id, attempt))
            .map_err(|error| error.to_string())?;
        let bindings = WorkAttemptWorkspaceBindings::new(work_run_id, attempt, roots, output);
        bindings.validate_attempt(work_run_id, attempt)?;
        Ok(bindings)
    }

    fn recover_work_attempt_bindings(
        &self,
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
        expected_roots: &[CoordinationRootBinding],
        expected_output: &DirId,
    ) -> Result<WorkAttemptWorkspaceBindings, String> {
        let mut roots = Vec::with_capacity(attempt.roots.len());
        for checkpoint in &attempt.roots {
            let owner = root_owner(work_run_id, attempt, checkpoint);
            let managed = self
                .worktree_runtime
                .block_on(self.worktrees.recover_owner(&owner))
                .map_err(|error| error.to_string())?;
            let source =
                Dir::open_local(managed.source_directory()).map_err(|error| error.to_string())?;
            validate_managed_root(checkpoint, &source, &managed)?;
            roots.push(WorkAttemptRootBinding {
                checkpoint: checkpoint.clone(),
                source,
                managed,
            });
        }
        let output = self
            .worktrees
            .recover_output(&output_owner(work_run_id, attempt))
            .map_err(|error| error.to_string())?;
        let bindings = WorkAttemptWorkspaceBindings::new(work_run_id, attempt, roots, output);
        let (actual_roots, actual_output) = bindings.coordination_bindings()?;
        if actual_roots != expected_roots || &actual_output != expected_output {
            return Err("durable WorkAttempt workspace does not match coordination state".into());
        }
        Ok(bindings)
    }

    fn source_root(
        &self,
        attempt: &WorkAttempt,
        checkpoint: &RootCheckpoint,
    ) -> Result<Dir, String> {
        if checkpoint.dir_id == self.dir_id {
            return Dir::open_local(&self.dir_root).map_err(|error| error.to_string());
        }
        self.file_access
            .list(&attempt.session_id)
            .into_iter()
            .find(|entry| {
                entry.dir().id() == checkpoint.dir_id
                    && entry.permissions().allows(Permission::InspectRepository)
            })
            .map(|entry| entry.dir().clone())
            .ok_or_else(|| {
                format!(
                    "source root {} is not available to Session {}",
                    checkpoint.dir_id, attempt.session_id
                )
            })
    }

    fn restore_default_thread_services(&self, thread_id: &zeta_protocol::ThreadId) {
        let Some(binding) = self.binding(thread_id) else {
            self.hooks.unbind_thread_dir(thread_id);
            return;
        };
        if let Ok(root) = Dir::open_local(binding.dir()) {
            let _ = self.hooks.bind_thread_dir(thread_id.clone(), root);
        }
        let _ = self.start_watcher(thread_id.clone(), [binding.dir().to_path_buf()]);
    }
}

impl WorkAttemptWorkspaceBindings {
    fn new(
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
        roots: Vec<WorkAttemptRootBinding>,
        output: ManagedOutputBinding,
    ) -> Self {
        Self {
            work_run_id: work_run_id.clone(),
            attempt_id: attempt.attempt_id.clone(),
            thread_id: attempt.thread_id.clone(),
            contract_id: attempt.contract.contract_id.clone(),
            contract_revision: attempt.contract.revision,
            primary_root_dir_id: attempt.primary_root_dir_id.clone(),
            roots,
            output,
        }
    }

    fn validate_attempt(
        &self,
        work_run_id: &WorkRunId,
        attempt: &WorkAttempt,
    ) -> Result<(), String> {
        if &self.work_run_id != work_run_id
            || self.attempt_id != attempt.attempt_id
            || self.thread_id != attempt.thread_id
            || self.contract_id != attempt.contract.contract_id
            || self.contract_revision != attempt.contract.revision
            || self.primary_root_dir_id != attempt.primary_root_dir_id
            || self
                .roots
                .iter()
                .map(|root| &root.checkpoint)
                .ne(attempt.roots.iter())
        {
            return Err(
                "WorkAttempt workspace identity does not match its immutable attempt".into(),
            );
        }
        Ok(())
    }

    fn coordination_bindings(&self) -> Result<(Vec<CoordinationRootBinding>, DirId), String> {
        let roots = self
            .roots
            .iter()
            .map(|root| {
                Ok(CoordinationRootBinding {
                    source_dir_id: root.checkpoint.dir_id.clone(),
                    managed_dir_id: Dir::open_local(root.managed.dir())
                        .map_err(|error| error.to_string())?
                        .id(),
                    root_checkpoint_digest: root_checkpoint_digest(&root.checkpoint)
                        .map_err(|error| error.to_string())?,
                    binding_manifest_digest: root
                        .managed
                        .manifest_digest()
                        .map_err(|error| error.to_string())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok((roots, self.output.dir_id().clone()))
    }

    fn primary_root(&self) -> Result<&WorkAttemptRootBinding, String> {
        self.roots
            .iter()
            .find(|root| root.checkpoint.dir_id == self.primary_root_dir_id)
            .ok_or_else(|| "WorkAttempt workspace omitted its primary root".into())
    }
}

fn root_owner(
    work_run_id: &WorkRunId,
    attempt: &WorkAttempt,
    checkpoint: &RootCheckpoint,
) -> ManagedDirOwner {
    ManagedDirOwner::WorkAttemptRoot {
        work_run_id: work_run_id.to_string(),
        attempt_id: attempt.attempt_id.to_string(),
        thread_id: attempt.thread_id.to_string(),
        source_dir_id: checkpoint.dir_id.to_string(),
    }
}

fn output_owner(work_run_id: &WorkRunId, attempt: &WorkAttempt) -> ManagedOutputOwner {
    ManagedOutputOwner::work_attempt(
        work_run_id.to_string(),
        attempt.attempt_id.to_string(),
        attempt.thread_id.to_string(),
    )
}

fn provision_request(
    checkpoint: &RootCheckpoint,
    source: &Dir,
    owner: ManagedDirOwner,
) -> Result<ManagedDirProvisionRequest, String> {
    let (source_state, target, repository_targets) = match &checkpoint.state {
        RootState::Git { repositories } => {
            let primary = repositories
                .iter()
                .find(|repository| Path::new(&repository.relative_path) == Path::new("."))
                .ok_or_else(|| "Git RootCheckpoint omitted its primary repository".to_string())?;
            let trees = repositories
                .iter()
                .map(|repository| {
                    (
                        PathBuf::from(&repository.relative_path),
                        repository.baseline_tree.clone(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let targets = repositories
                .iter()
                .filter(|repository| repository.relative_path != ".")
                .map(|repository| {
                    Ok((
                        PathBuf::from(&repository.relative_path),
                        managed_target(&repository.target),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            (
                ManagedDirSource::ImmutableTree {
                    source_directory: source.canonical_path().to_path_buf(),
                    tree_id: primary.baseline_tree.clone(),
                    repository_trees: trees,
                },
                managed_target(&primary.target),
                targets,
            )
        }
        RootState::Directory { snapshot_id } => (
            ManagedDirSource::ImmutableTree {
                source_directory: source.canonical_path().to_path_buf(),
                tree_id: snapshot_id.clone(),
                repository_trees: BTreeMap::new(),
            },
            ManagedDirTarget::SourceHead,
            BTreeMap::new(),
        ),
    };
    Ok(ManagedDirProvisionRequest {
        source: source_state,
        target,
        repository_targets,
        source_dir_id: checkpoint.dir_id.to_string(),
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

fn validate_managed_root(
    checkpoint: &RootCheckpoint,
    source: &Dir,
    binding: &ManagedDirBinding,
) -> Result<(), String> {
    if source.id() != checkpoint.dir_id || binding.source_dir_id() != checkpoint.dir_id.as_str() {
        return Err("managed root source identity does not match its RootCheckpoint".into());
    }
    match &checkpoint.state {
        RootState::Directory { snapshot_id } => {
            if binding.kind() != ManagedDirKind::Directory
                || binding.baseline_tree() != snapshot_id
                || binding.repositories().len() != 1
            {
                return Err("managed directory does not match its immutable snapshot".into());
            }
        }
        RootState::Git { repositories } => {
            if binding.kind() != ManagedDirKind::Git
                || binding.repositories().len() != repositories.len()
            {
                return Err("managed Git root repository set does not match its checkpoint".into());
            }
            for checkpoint_repository in repositories {
                let repository = binding
                    .repositories()
                    .iter()
                    .find(|repository| {
                        repository.relative_path()
                            == Path::new(&checkpoint_repository.relative_path)
                    })
                    .ok_or_else(|| {
                        format!(
                            "managed root omitted repository {}",
                            checkpoint_repository.relative_path
                        )
                    })?;
                validate_repository(checkpoint_repository, repository)?;
            }
        }
    }
    Ok(())
}

fn validate_repository(
    checkpoint: &GitRepositoryCheckpoint,
    binding: &zeta_worktree::ManagedRepositoryBinding,
) -> Result<(), String> {
    if checkpoint.repository_id != binding.repository_id()
        || checkpoint.baseline_tree != binding.baseline_tree()
    {
        return Err("managed repository identity or baseline does not match its checkpoint".into());
    }
    let target_matches = match &checkpoint.target {
        GitRootTarget::Branch {
            name,
            expected_head,
        } => {
            binding.target_branch() == Some(name.as_str())
                && !binding.target_unborn()
                && binding.target_head() == expected_head
        }
        GitRootTarget::UnbornBranch {
            name,
            anchor_object_id,
        } => {
            binding.target_branch() == Some(name.as_str())
                && binding.target_unborn()
                && binding.target_head() == anchor_object_id
        }
        GitRootTarget::Detached { object_id } => {
            binding.target_branch().is_none()
                && !binding.target_unborn()
                && binding.target_head() == object_id
        }
    };
    if !target_matches {
        return Err("managed repository target does not match its checkpoint".into());
    }
    Ok(())
}
