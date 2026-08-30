use super::turn_changes_runtime::TurnChangesRuntime;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use zeta_core::{CoreError, ThreadWorktreeBinder, ThreadWorktreeBindingRequest};
use zeta_file_access::Dir;
use zeta_protocol::ThreadOrigin;
use zeta_turn_changes::{CommitState, TurnChangeSet, TurnChangeStore};
use zeta_worktree::{
    ThreadRepositoryBinding, ThreadWorktreeBinding, ThreadWorktreeKind,
    ThreadWorktreeProvisionRequest, ThreadWorktreeSource, ThreadWorktreeTarget,
};

impl TurnChangesRuntime {
    pub(super) fn enforce_cleanup_policy(&self) -> Result<(), String> {
        let settings = self.worktrees.settings();
        if !settings.auto_cleanup_enabled {
            return Ok(());
        }
        let bindings = self
            .bindings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .map(|(thread_id, binding)| (thread_id.clone(), binding.clone()))
            .collect::<Vec<_>>();
        let mut eligible = Vec::new();
        for (thread_id, binding) in bindings {
            let Ok(thread) = self.threads.read_thread(&thread_id) else {
                continue;
            };
            if thread.status == zeta_protocol::ThreadStatus::Active {
                continue;
            }
            let records = self
                .store
                .list_for_thread(&thread_id)
                .map_err(|error| error.to_string())?;
            let settled = records.iter().all(|record| {
                record.files.is_empty()
                    || record.capture_state == zeta_turn_changes::CaptureState::Discarded
                    || matches!(record.commit_state, CommitState::Committed { .. })
            });
            if !settled {
                continue;
            }
            let modified = std::fs::metadata(binding.checkout_root())
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            eligible.push((modified, thread_id, binding));
        }
        eligible.sort_by(|left, right| right.0.cmp(&left.0));
        for (_, thread_id, binding) in eligible.into_iter().skip(settings.keep_count) {
            self.worktree_runtime
                .block_on(self.worktrees.cleanup_thread(
                    &binding,
                    zeta_worktree::ThreadWorktreeCleanupEligibility::AllChangeSetsSettled,
                ))
                .map_err(|error| error.to_string())?;
            self.bindings
                .write()
                .map_err(|_| "Thread dir binding lock poisoned".to_string())?
                .remove(&thread_id);
            self.file_access.unbind_thread_dir(&thread_id);
            self.hooks.unbind_thread_dir(&thread_id);
            self.stop_watcher(&thread_id);
        }
        Ok(())
    }

    pub(super) fn reset_thread_to_committed_changes(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        records: &[TurnChangeSet],
    ) -> Result<(), String> {
        let binding = self
            .binding(thread_id)
            .ok_or_else(|| format!("Thread {thread_id} has no dir binding"))?;
        self.worktree_runtime.block_on(async {
            if binding.kind() == ThreadWorktreeKind::Directory {
                let object_store = binding
                    .snapshot_store()
                    .ok_or_else(|| "managed directory omitted its snapshot store".to_string())?;
                let snapshots = zeta_turn_changes::DirectorySnapshotStore::new(object_store);
                let mut desired = binding.baseline_tree().to_string();
                for record in records {
                    if !matches!(record.commit_state, CommitState::Committed { .. }) {
                        continue;
                    }
                    let after = record
                        .after_tree
                        .as_deref()
                        .ok_or_else(|| "committed ChangeSet omitted its after tree".to_string())?;
                    desired = match snapshots.replay(&record.before_tree, &desired, after)? {
                        zeta_turn_changes::DirectoryReplayResult::Clean(tree) => tree,
                        zeta_turn_changes::DirectoryReplayResult::Conflict(paths) => {
                            return Err(format!(
                                "committed Thread history cannot be reconstructed: {}",
                                paths
                                    .iter()
                                    .map(|path| path.display().to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                    };
                }
                return snapshots.replace_directory(binding.dir(), &desired);
            }
            let git = zeta_git::GitClient::system();
            for repository_binding in binding.repositories() {
                let repository = git
                    .open_repository(repository_binding.worktree_root())
                    .await
                    .map_err(|error| error.to_string())?;
                let mut desired =
                    zeta_git::GitTreeId::new(repository_binding.baseline_tree().to_string())
                        .map_err(|error| error.to_string())?;
                for record in records.iter().filter(|record| {
                    record.repository_id == repository_binding.repository_id()
                        && matches!(record.commit_state, CommitState::Committed { .. })
                }) {
                    let before = zeta_git::GitTreeId::new(record.before_tree.clone())
                        .map_err(|error| error.to_string())?;
                    let after =
                        zeta_git::GitTreeId::new(record.after_tree.clone().ok_or_else(|| {
                            "committed ChangeSet omitted its after tree".to_string()
                        })?)
                        .map_err(|error| error.to_string())?;
                    desired = git
                        .compose_tree_delta(&repository, &before, &desired, &after)
                        .await
                        .map_err(|error| error.to_string())?;
                }
                git.replace_managed_worktree_tree(&repository, &desired)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })
    }

    pub(super) fn initial_baseline_paths(
        &self,
        binding: &ThreadWorktreeBinding,
        repository_binding: &ThreadRepositoryBinding,
    ) -> Result<BTreeSet<PathBuf>, CoreError> {
        if binding.kind() == ThreadWorktreeKind::Directory {
            return Ok(BTreeSet::new());
        }
        self.worktree_runtime.block_on(async {
            let git = zeta_git::GitClient::system();
            let repository = git
                .open_repository(repository_binding.worktree_root())
                .await
                .map_err(|error| CoreError::Journal(error.to_string()))?;
            let target_tree = if repository_binding.target_unborn() {
                git.empty_tree(&repository)
                    .await
                    .map_err(|error| CoreError::Journal(error.to_string()))?
            } else {
                git.resolve_tree(&repository, repository_binding.target_head())
                    .await
                    .map_err(|error| CoreError::Journal(error.to_string()))?
            };
            let baseline_tree =
                zeta_git::GitTreeId::new(repository_binding.baseline_tree().to_string())
                    .map_err(|error| CoreError::Journal(error.to_string()))?;
            let changes = git
                .diff_trees(&repository, &target_tree, &baseline_tree)
                .await
                .map_err(|error| CoreError::Journal(error.to_string()))?;
            Ok(changes
                .into_iter()
                .flat_map(|change| {
                    [
                        Some(change.path().to_path_buf()),
                        change.previous_path().map(PathBuf::from),
                    ]
                })
                .flatten()
                .collect())
        })
    }

    fn source_for(
        &self,
        origin: &ThreadOrigin,
    ) -> Result<(ThreadWorktreeSource, ThreadWorktreeTarget), CoreError> {
        let parent_id = match origin {
            ThreadOrigin::Root => {
                return Ok((
                    ThreadWorktreeSource::DirSnapshot {
                        source_directory: self.dir_root.clone(),
                    },
                    ThreadWorktreeTarget::SourceHead,
                ));
            }
            ThreadOrigin::Fork {
                parent_thread_id, ..
            }
            | ThreadOrigin::Rewind {
                parent_thread_id, ..
            }
            | ThreadOrigin::AgentSpawn {
                parent_thread_id, ..
            } => parent_thread_id,
        };
        let parent = self.binding(parent_id).ok_or_else(|| {
            CoreError::Journal(format!("parent Thread {parent_id} has no dir binding"))
        })?;
        let source = match origin {
            ThreadOrigin::Rewind { before_turn_id, .. } => {
                let records = self
                    .store
                    .list_for_thread(parent_id)
                    .map_err(|error| CoreError::Journal(error.to_string()))?
                    .into_iter()
                    .filter(|record| record.turn_id == *before_turn_id)
                    .collect::<Vec<_>>();
                if records.is_empty() {
                    return Err(CoreError::Journal(format!(
                        "rewind Turn {before_turn_id} has no immutable dir checkpoint"
                    )));
                }
                let trees = checkpoint_trees(&parent, &records, false).ok_or_else(|| {
                    CoreError::Journal(format!(
                        "rewind Turn {before_turn_id} has an incomplete repository checkpoint"
                    ))
                })?;
                ThreadWorktreeSource::ImmutableTree {
                    source_directory: parent.dir().to_path_buf(),
                    tree_id: primary_tree(&trees)?,
                    repository_trees: trees,
                }
            }
            ThreadOrigin::Fork {
                parent_sequence, ..
            } => {
                let trees = self.trees_at_sequence(parent_id, *parent_sequence)?;
                ThreadWorktreeSource::ImmutableTree {
                    source_directory: parent.dir().to_path_buf(),
                    tree_id: primary_tree(&trees)?,
                    repository_trees: trees,
                }
            }
            ThreadOrigin::AgentSpawn { .. } => ThreadWorktreeSource::DirSnapshot {
                source_directory: parent.dir().to_path_buf(),
            },
            ThreadOrigin::Root => unreachable!("root source returned above"),
        };
        let target = match (parent.target_branch(), parent.target_unborn()) {
            (Some(name), true) => ThreadWorktreeTarget::UnbornBranch {
                name: name.to_string(),
                anchor_object_id: parent.target_head().to_string(),
            },
            (Some(name), false) => ThreadWorktreeTarget::Branch {
                name: name.to_string(),
                object_id: parent.target_head().to_string(),
            },
            (None, _) => ThreadWorktreeTarget::Detached {
                object_id: parent.target_head().to_string(),
            },
        };
        Ok((source, target))
    }

    fn trees_at_sequence(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        sequence: u64,
    ) -> Result<BTreeMap<PathBuf, String>, CoreError> {
        let updates = self
            .threads
            .thread_updates_after(thread_id, 0)
            .map_err(|error| CoreError::Journal(error.to_string()))?;
        let mut last_terminal_turn = None;
        for update in updates {
            if update.durable_sequence > sequence {
                break;
            }
            let zeta_protocol::ThreadUpdate::Committed { event } = update.update else {
                continue;
            };
            match event {
                zeta_protocol::ThreadEvent::TurnCompleted { turn_id, .. }
                | zeta_protocol::ThreadEvent::TurnFailed { turn_id, .. }
                | zeta_protocol::ThreadEvent::TurnInterrupted { turn_id, .. } => {
                    last_terminal_turn = Some(turn_id);
                }
                _ => {}
            }
        }
        let Some(turn_id) = last_terminal_turn else {
            return self
                .binding(thread_id)
                .map(|binding| {
                    binding
                        .repositories()
                        .iter()
                        .map(|repository| {
                            (
                                repository.relative_path().to_path_buf(),
                                repository.baseline_tree().to_string(),
                            )
                        })
                        .collect()
                })
                .ok_or_else(|| CoreError::Journal(format!("Thread {thread_id} has no baseline")));
        };
        let binding = self
            .binding(thread_id)
            .ok_or_else(|| CoreError::Journal(format!("Thread {thread_id} has no binding")))?;
        let records = self
            .store
            .list_for_thread(thread_id)
            .map_err(|error| CoreError::Journal(error.to_string()))?
            .into_iter()
            .filter(|record| record.turn_id == turn_id)
            .collect::<Vec<_>>();
        checkpoint_trees(&binding, &records, true).ok_or_else(|| {
            CoreError::Journal(format!(
                "Fork source Turn {turn_id} has no complete sealed dir checkpoint"
            ))
        })
    }
}

fn checkpoint_trees(
    binding: &ThreadWorktreeBinding,
    records: &[TurnChangeSet],
    after: bool,
) -> Option<BTreeMap<PathBuf, String>> {
    binding
        .repositories()
        .iter()
        .map(|repository| {
            let record = records
                .iter()
                .find(|record| record.repository_id == repository.repository_id())?;
            let tree = if after {
                record.after_tree.clone()?
            } else {
                record.before_tree.clone()
            };
            Some((repository.relative_path().to_path_buf(), tree))
        })
        .collect()
}

fn primary_tree(trees: &BTreeMap<PathBuf, String>) -> Result<String, CoreError> {
    trees.get(Path::new(".")).cloned().ok_or_else(|| {
        CoreError::Journal("Thread checkpoint omitted its primary repository".into())
    })
}

impl ThreadWorktreeBinder for TurnChangesRuntime {
    fn provision(&self, request: &ThreadWorktreeBindingRequest) -> Result<(), CoreError> {
        if let Some(binding) = self.binding(&request.thread_id) {
            self.bind_thread_services(&request.thread_id, &binding)?;
            return Ok(());
        }
        let (source, target) = self.source_for(&request.origin)?;
        let binding = self
            .worktree_runtime
            .block_on(
                self.worktrees
                    .provision_thread(&ThreadWorktreeProvisionRequest {
                        source,
                        target,
                        source_dir_id: self.dir_id.to_string(),
                        thread_id: request.thread_id.to_string(),
                    }),
            )
            .map_err(|error| CoreError::Journal(format!("cannot provision Thread dir: {error}")))?;
        self.bindings
            .write()
            .map_err(|_| CoreError::Journal("Thread dir binding lock poisoned".into()))?
            .insert(request.thread_id.clone(), binding);
        let binding = self.binding(&request.thread_id).ok_or_else(|| {
            CoreError::Journal("Thread dir binding disappeared after provision".into())
        })?;
        self.bind_thread_services(&request.thread_id, &binding)
    }
}

impl TurnChangesRuntime {
    fn bind_thread_services(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        binding: &ThreadWorktreeBinding,
    ) -> Result<(), CoreError> {
        let root = Dir::open_local(binding.dir())
            .map_err(|error| CoreError::Journal(error.to_string()))?;
        self.hooks
            .bind_thread_dir(thread_id.clone(), root.clone())
            .map_err(|error| CoreError::Journal(error.to_string()))?;
        self.file_access.bind_thread_dir(thread_id.clone(), root);
        self.start_watcher(thread_id.clone(), binding.dir())
            .map_err(CoreError::Journal)?;
        Ok(())
    }
}
