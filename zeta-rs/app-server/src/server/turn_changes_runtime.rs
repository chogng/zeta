use super::turn_changes_commit::settle_dependencies;
use super::turn_changes_commit::spawn_commit_job;
use super::turn_changes_message::spawn_message_job;
use super::turn_changes_watcher::ThreadChangeWatcher;
use super::update_broker::UpdateBroker;
use crate::dir_grants::DirGrants;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use zeta_app_server_protocol::protocol::turn_changes::{
    ChangeSetId as ChangeSetIdDto, ThreadDirBinding, ThreadWorktreeRepositoryBindingDto,
    TurnChangeCaptureStateDto, TurnChangeCommitStateDto, TurnChangeFileStatisticsDto,
    TurnChangeMessageStateDto, TurnChangeSetSummary, TurnChangeTerminalStateDto,
    TurnChangesChanged, TurnChangesMutationResult,
};
use zeta_config::ConfigStore;
use zeta_core::{ModelService, ThreadController};
use zeta_file_access::{Dir, DirId};
use zeta_hooks::DeclarativeHookRuntime;
use zeta_protocol::{CommandId, SessionId, ThreadId, ToolCallId, TurnId};
use zeta_state::{SqliteTurnChangeStore, TurnChangeCommandOutcome};
use zeta_turn_changes::{
    CaptureState, CommitState, MessageState, TerminalTurnState, TurnChangeLedger, TurnChangeSet,
    TurnChangeStore,
};
use zeta_worktree::{ThreadWorktreeBinding, WorktreeManager, WorktreeSettings};

/// App Server owner of Thread dir bindings, Turn checkpoints, and ledger notifications.
pub(super) struct TurnChangesRuntime {
    pub(super) dir_root: PathBuf,
    pub(super) dir_id: DirId,
    pub(super) worktrees: WorktreeManager,
    pub(super) worktree_runtime: tokio::runtime::Runtime,
    pub(super) bindings: RwLock<BTreeMap<ThreadId, ThreadWorktreeBinding>>,
    pub(super) store: Arc<SqliteTurnChangeStore>,
    pub(super) ledger: TurnChangeLedger,
    pub(super) config: Arc<ConfigStore>,
    pub(super) threads: Arc<ThreadController>,
    pub(super) model: Arc<dyn ModelService>,
    pub(super) file_access: Arc<DirGrants>,
    pub(super) hooks: Arc<DeclarativeHookRuntime>,
    pub(super) updates: Arc<UpdateBroker>,
    pub(super) capture_failures: RwLock<BTreeMap<TurnId, String>>,
    pub(super) tool_write_capabilities: RwLock<BTreeMap<(TurnId, ToolCallId), bool>>,
    pub(super) active_write_lifecycles: Arc<RwLock<BTreeMap<(ThreadId, TurnId), usize>>>,
    pub(super) watchers: RwLock<BTreeMap<ThreadId, ThreadChangeWatcher>>,
}

impl TurnChangesRuntime {
    pub(super) fn open(
        database_path: &Path,
        profile_root: &Path,
        dir_root: &Path,
        config: Arc<ConfigStore>,
        threads: Arc<ThreadController>,
        model: Arc<dyn ModelService>,
        file_access: Arc<DirGrants>,
        hooks: Arc<DeclarativeHookRuntime>,
        updates: Arc<UpdateBroker>,
    ) -> Result<Arc<Self>, String> {
        let dir = Dir::open_local(dir_root).map_err(|error| error.to_string())?;
        let store = Arc::new(
            SqliteTurnChangeStore::open(database_path)
                .map_err(|error| format!("cannot open Turn change ledger: {error}"))?,
        );
        let ledger_store: Arc<dyn TurnChangeStore> = store.clone();
        let ledger = TurnChangeLedger::start(ledger_store).map_err(|error| error.to_string())?;
        let worktree_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .thread_name("zeta-thread-worktrees")
            .build()
            .map_err(|error| error.to_string())?;
        let desktop = config
            .read_snapshot()
            .map_err(|error| format!("cannot read managed worktree settings: {error}"))?
            .values
            .desktop;
        let worktree_settings = WorktreeSettings::from_desktop_config(profile_root, &desktop)
            .map_err(|error| format!("cannot resolve managed worktree settings: {error}"))?;
        let worktrees = WorktreeManager::new(worktree_settings);
        let recovered = worktree_runtime
            .block_on(worktrees.recover_threads(dir.canonical_path(), dir.id().as_str()))
            .map_err(|error| format!("cannot recover Thread worktrees: {error}"))?;
        let bindings = recovered
            .into_iter()
            .map(|(thread_id, binding)| {
                ThreadId::new(thread_id)
                    .map(|thread_id| (thread_id, binding))
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        for (thread_id, binding) in &bindings {
            let root = Dir::open_local(binding.dir()).map_err(|error| error.to_string())?;
            hooks
                .bind_thread_dir(thread_id.clone(), root.clone())
                .map_err(|error| error.to_string())?;
            file_access.bind_thread_dir(thread_id.clone(), root);
        }
        let runtime = Arc::new(Self {
            dir_root: dir.canonical_path().to_path_buf(),
            dir_id: dir.id(),
            worktrees,
            worktree_runtime,
            bindings: RwLock::new(bindings),
            store,
            ledger,
            config,
            threads,
            model,
            file_access,
            hooks: Arc::clone(&hooks),
            updates,
            capture_failures: RwLock::new(BTreeMap::new()),
            tool_write_capabilities: RwLock::new(BTreeMap::new()),
            active_write_lifecycles: Arc::new(RwLock::new(BTreeMap::new())),
            watchers: RwLock::new(BTreeMap::new()),
        });
        let hook_observer: Arc<dyn zeta_core::HookExecutionObserver> = runtime.clone();
        hooks.set_execution_observer(hook_observer);
        for (thread_id, binding) in runtime
            .bindings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
        {
            runtime.start_watcher(thread_id.clone(), binding.dir())?;
        }
        runtime.resume_pending_jobs()?;
        Ok(runtime)
    }

    pub(super) fn store(&self) -> &Arc<SqliteTurnChangeStore> {
        &self.store
    }

    pub(super) fn binding(&self, thread_id: &ThreadId) -> Option<ThreadWorktreeBinding> {
        self.bindings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(thread_id)
            .cloned()
    }

    fn resume_pending_jobs(self: &Arc<Self>) -> Result<(), String> {
        let thread_ids = self
            .bindings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for thread_id in thread_ids {
            let records = self
                .store
                .list_for_thread(&thread_id)
                .map_err(|error| error.to_string())?;
            for committed in records
                .iter()
                .filter(|record| matches!(record.commit_state, CommitState::Committed { .. }))
            {
                settle_dependencies(&self.store, committed)?;
            }
            for record in records {
                if matches!(
                    record.message_state,
                    MessageState::Queued | MessageState::Generating
                ) && !record.files.is_empty()
                {
                    spawn_message_job(
                        Arc::clone(&self.store),
                        Arc::clone(&self.threads),
                        Arc::clone(&self.model),
                        Arc::clone(&self.config),
                        self.dir_id.clone(),
                        Arc::clone(&self.updates),
                        record.change_set_id.clone(),
                    );
                }
                if matches!(
                    record.commit_state,
                    CommitState::Queued | CommitState::Committing
                ) {
                    let binding = self
                        .binding(&record.thread_id)
                        .ok_or_else(|| format!("Thread {} has no dir binding", record.thread_id))?;
                    spawn_commit_job(
                        Arc::clone(&self.store),
                        Arc::clone(&self.updates),
                        binding,
                        record.change_set_id,
                    );
                }
            }
        }
        Ok(())
    }

    pub(super) fn public_binding(&self, thread_id: &ThreadId) -> Option<ThreadDirBinding> {
        self.binding(thread_id).map(|binding| ThreadDirBinding {
            managed_worktree_id: binding.managed_worktree_id().to_string(),
            source_dir_id: binding.source_dir_id().to_string(),
            repositories: binding
                .repositories()
                .iter()
                .map(|repository| ThreadWorktreeRepositoryBindingDto {
                    repository_id: repository.repository_id().to_string(),
                    target_branch: repository.target_branch().map(ToOwned::to_owned),
                    baseline_object_id: (!repository.target_unborn())
                        .then(|| repository.baseline_tree().to_string()),
                })
                .collect(),
            baseline_summary: format!(
                "{} immutable repository checkpoint(s)",
                binding.repositories().len()
            ),
        })
    }

    pub(super) fn list(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<Vec<TurnChangeSet>, String> {
        let records = self
            .store
            .list_for_thread(thread_id)
            .map_err(|error| error.to_string())?;
        if records
            .iter()
            .any(|record| &record.session_id != session_id)
        {
            return Err("Thread change ledger ownership does not match Session".into());
        }
        let mut open_turns = Vec::new();
        for record in &records {
            if record.capture_state == CaptureState::Open && !open_turns.contains(&record.turn_id) {
                open_turns.push(record.turn_id.clone());
            }
        }
        for turn_id in open_turns {
            let refreshed = self
                .ledger
                .refresh_turn(session_id.clone(), thread_id.clone(), turn_id)
                .map_err(|error| error.to_string())?;
            self.publish(&refreshed);
        }
        self.store
            .list_for_thread(thread_id)
            .map_err(|error| error.to_string())
    }

    pub(super) fn retry_message(
        self: &Arc<Self>,
        mut record: TurnChangeSet,
        expected_revision: u64,
        command_id: &CommandId,
        fingerprint: &str,
    ) -> Result<TurnChangesMutationResult, String> {
        require_revision(&record, expected_revision)?;
        record.queue_message().map_err(|error| error.to_string())?;
        let response = mutation_result(&[record.clone()]);
        if let Some(replayed) = self.apply_command(command_id, fingerprint, &record, &response)? {
            return Ok(replayed);
        }
        self.publish(&[record.clone()]);
        spawn_message_job(
            Arc::clone(&self.store),
            Arc::clone(&self.threads),
            Arc::clone(&self.model),
            Arc::clone(&self.config),
            self.dir_id.clone(),
            Arc::clone(&self.updates),
            record.change_set_id.clone(),
        );
        Ok(response)
    }

    pub(super) fn update_draft(
        &self,
        mut record: TurnChangeSet,
        expected_revision: u64,
        message: String,
        command_id: &CommandId,
        fingerprint: &str,
    ) -> Result<TurnChangesMutationResult, String> {
        require_revision(&record, expected_revision)?;
        record
            .update_draft(message)
            .map_err(|error| error.to_string())?;
        let response = mutation_result(&[record.clone()]);
        if let Some(replayed) = self.apply_command(command_id, fingerprint, &record, &response)? {
            return Ok(replayed);
        }
        self.publish(&[record]);
        Ok(response)
    }

    pub(super) fn queue_commit(
        self: &Arc<Self>,
        mut record: TurnChangeSet,
        expected_revision: u64,
        command_id: &CommandId,
        fingerprint: &str,
    ) -> Result<TurnChangesMutationResult, String> {
        require_revision(&record, expected_revision)?;
        if record.target_branch.is_none() {
            return Err("detached Thread targets cannot be committed".into());
        }
        let binding = self
            .binding(&record.thread_id)
            .ok_or_else(|| format!("Thread {} has no dir binding", record.thread_id))?;
        self.resolve_external_dependencies(&binding, &mut record)?;
        record.queue_commit().map_err(|error| error.to_string())?;
        let response = mutation_result(&[record.clone()]);
        if let Some(replayed) = self.apply_command(command_id, fingerprint, &record, &response)? {
            return Ok(replayed);
        }
        self.publish(&[record.clone()]);
        spawn_commit_job(
            Arc::clone(&self.store),
            Arc::clone(&self.updates),
            binding,
            record.change_set_id,
        );
        Ok(response)
    }

    fn resolve_external_dependencies(
        &self,
        binding: &ThreadWorktreeBinding,
        record: &mut TurnChangeSet,
    ) -> Result<(), String> {
        if record.external_dependency_paths.is_empty() {
            return Ok(());
        }
        let repository_binding = binding
            .repositories()
            .iter()
            .find(|repository| repository.repository_id() == record.repository_id)
            .ok_or_else(|| format!("Thread binding omitted repository {}", record.repository_id))?;
        if binding.kind() != zeta_worktree::ThreadWorktreeKind::Git {
            return Ok(());
        }
        let resolved = self.worktree_runtime.block_on(async {
            let git = zeta_git::GitClient::system();
            let repository = git
                .open_repository(repository_binding.source_repository_root())
                .await
                .map_err(|error| error.to_string())?;
            let Some(branch_name) = record.target_branch.as_deref() else {
                return Ok(Vec::new());
            };
            let Some(branch) = git
                .local_branches(&repository)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .find(|branch| branch.name() == branch_name)
            else {
                return Ok(Vec::new());
            };
            let current = git
                .resolve_tree(&repository, branch.object_id())
                .await
                .map_err(|error| error.to_string())?;
            let baseline = zeta_git::GitTreeId::new(record.before_tree.clone())
                .map_err(|error| error.to_string())?;
            let differences = git
                .diff_trees(&repository, &current, &baseline)
                .await
                .map_err(|error| error.to_string())?;
            Ok::<_, String>(
                record
                    .external_dependency_paths
                    .iter()
                    .filter(|dependency| {
                        differences.iter().all(|change| {
                            !paths_overlap(dependency, change.path())
                                && change
                                    .previous_path()
                                    .is_none_or(|path| !paths_overlap(dependency, path))
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        })?;
        record
            .satisfy_external_dependencies(resolved)
            .map_err(|error| error.to_string())
    }

    fn apply_command(
        &self,
        command_id: &CommandId,
        fingerprint: &str,
        record: &TurnChangeSet,
        response: &TurnChangesMutationResult,
    ) -> Result<Option<TurnChangesMutationResult>, String> {
        let response_json = serde_json::to_string(response).map_err(|error| error.to_string())?;
        match self
            .store
            .apply_command(
                command_id.as_str(),
                fingerprint,
                None,
                &[record.clone()],
                &response_json,
            )
            .map_err(|error| error.to_string())?
        {
            TurnChangeCommandOutcome::Applied => Ok(None),
            TurnChangeCommandOutcome::Replayed(response) => serde_json::from_str(&response)
                .map(Some)
                .map_err(|error| error.to_string()),
        }
    }

    pub(super) fn publish(&self, records: &[TurnChangeSet]) {
        publish_records(&self.updates, records);
    }

    pub(super) fn commit_message_configured(&self) -> bool {
        self.config.read_snapshot().is_ok_and(|snapshot| {
            snapshot
                .values
                .commit_messages
                .authorized_model(
                    &self.dir_id,
                    snapshot.values.commit_message_model.as_ref(),
                    &snapshot.values.providers,
                )
                .is_some()
        })
    }
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn require_revision(record: &TurnChangeSet, expected_revision: u64) -> Result<(), String> {
    if record.revision == expected_revision {
        Ok(())
    } else {
        Err(format!(
            "change-set revision conflict: expected {expected_revision}, actual {}",
            record.revision
        ))
    }
}

fn mutation_result(records: &[TurnChangeSet]) -> TurnChangesMutationResult {
    TurnChangesMutationResult {
        change_sets: records.iter().map(summary).collect(),
    }
}

pub(super) fn publish_records(updates: &UpdateBroker, records: &[TurnChangeSet]) {
    let Some(first) = records.first() else {
        return;
    };
    updates.publish_turn_changes_changed(TurnChangesChanged {
        session_id: first.session_id.clone(),
        thread_id: first.thread_id.clone(),
        change_sets: records.iter().map(summary).collect(),
    });
}

pub(super) fn summary(record: &TurnChangeSet) -> TurnChangeSetSummary {
    let (commit_state, conflict_paths, failure_message, commit_id) = match &record.commit_state {
        CommitState::Idle => (TurnChangeCommitStateDto::Idle, Vec::new(), None, None),
        CommitState::Queued => (TurnChangeCommitStateDto::Queued, Vec::new(), None, None),
        CommitState::Committing => (TurnChangeCommitStateDto::Committing, Vec::new(), None, None),
        CommitState::Committed { object_id } => (
            TurnChangeCommitStateDto::Committed,
            Vec::new(),
            None,
            Some(object_id.clone()),
        ),
        CommitState::Conflict { paths } => (
            TurnChangeCommitStateDto::Conflict,
            paths
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            None,
            None,
        ),
        CommitState::Failed { message } => (
            TurnChangeCommitStateDto::Failed,
            Vec::new(),
            Some(message.clone()),
            None,
        ),
    };
    TurnChangeSetSummary {
        change_set_id: ChangeSetIdDto(record.change_set_id.to_string()),
        session_id: record.session_id.clone(),
        thread_id: record.thread_id.clone(),
        turn_id: record.turn_id.clone(),
        repository_id: record.repository_id.clone(),
        target_branch: record.target_branch.clone(),
        statistics: TurnChangeFileStatisticsDto {
            files: record.files.len() as u64,
            additions: record.files.iter().map(|file| file.additions).sum(),
            deletions: record.files.iter().map(|file| file.deletions).sum(),
        },
        capture_state: match record.capture_state {
            CaptureState::Open => TurnChangeCaptureStateDto::Open,
            CaptureState::Sealed => TurnChangeCaptureStateDto::Sealed,
            CaptureState::Incomplete => TurnChangeCaptureStateDto::Incomplete,
            CaptureState::Discarded => TurnChangeCaptureStateDto::Discarded,
        },
        message_state: match record.message_state {
            MessageState::Unconfigured => TurnChangeMessageStateDto::Unconfigured,
            MessageState::Queued => TurnChangeMessageStateDto::Queued,
            MessageState::Generating => TurnChangeMessageStateDto::Generating,
            MessageState::Ready => TurnChangeMessageStateDto::Ready,
            MessageState::Failed => TurnChangeMessageStateDto::Failed,
        },
        commit_state,
        terminal_state: record.terminal_state.map(|state| match state {
            TerminalTurnState::Completed => TurnChangeTerminalStateDto::Completed,
            TerminalTurnState::Failed => TurnChangeTerminalStateDto::Failed,
            TerminalTurnState::Interrupted => TurnChangeTerminalStateDto::Interrupted,
        }),
        dependencies: record
            .dependencies
            .iter()
            .map(|id| ChangeSetIdDto(id.to_string()))
            .collect(),
        external_dependency_paths: record
            .external_dependency_paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        warnings: record.warnings.clone(),
        conflict_paths,
        failure_message,
        commit_id,
        revision: record.revision,
    }
}
