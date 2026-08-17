use super::update_broker::UpdateBroker;
use crate::git_service::{GitService, GitServiceCommit, GitServiceError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_app_server_protocol::protocol::git::{
    GitBranchDto, GitChangeStatusDto, GitCommitSummaryDto, GitDiffStatisticsDto, GitGraphResult,
    GitHeadDto, GitReferenceDto, GitReferenceKindDto, GitRemoteDto, GitRemoteProviderDto,
    GitRepositoryChangeDto, GitRepositoryIdentityDto, GitStatusResult, GitSubmoduleStateDto,
    GitTextDiffDto, GitTextDiffResult, GitUpstreamDto,
};
use zeta_file_watcher::{DebouncedWatchReceiver, FileWatcher, FileWatcherBackend, WatchPath};
use zeta_git::{
    GitChangeStatus, GitGraph, GitHead, GitReferenceKind, GitRemoteProvider, GitRepository,
    GitRepositoryChange, GitRepositorySnapshot,
};
use zeta_protocol::StreamInstanceId;
use zeta_workspace::TrustedWorkspace;

const GIT_WATCH_DEBOUNCE: Duration = Duration::from_millis(100);
const ALIASED_PATH_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) struct GitRuntime {
    service: GitService,
    stream_instance_id: StreamInstanceId,
    operation: Mutex<()>,
    state: Mutex<GitRuntimeState>,
    updates: Arc<UpdateBroker>,
}

#[derive(Default)]
pub(super) struct GitWatcher {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

struct GitRuntimeState {
    revision: u64,
    repository: Option<GitRepository>,
    status: Option<GitStatusResult>,
}

pub(super) struct GitRuntimeCommit {
    pub(super) object_id: String,
    pub(super) status: GitStatusResult,
}

#[derive(Debug)]
pub(crate) enum GitRuntimeError {
    Boundary,
    Service(GitServiceError),
}

impl GitRuntime {
    pub(super) fn new(
        workspace: TrustedWorkspace,
        updates: Arc<UpdateBroker>,
    ) -> Result<Arc<Self>, GitRuntimeError> {
        Ok(Arc::new(Self {
            service: GitService::new(workspace).map_err(GitRuntimeError::Service)?,
            stream_instance_id: new_stream_instance_id()?,
            operation: Mutex::new(()),
            state: Mutex::new(GitRuntimeState {
                revision: 0,
                repository: None,
                status: None,
            }),
            updates,
        }))
    }

    pub(super) fn status(&self) -> Result<GitStatusResult, GitRuntimeError> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        let (repository, snapshot) = self.service.snapshot().map_err(GitRuntimeError::Service)?;
        self.accept(repository, snapshot)
    }

    pub(super) fn local_branches(&self) -> Result<Vec<GitBranchDto>, GitRuntimeError> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        self.service
            .local_branches()
            .map(|branches| {
                branches
                    .into_iter()
                    .map(|branch| GitBranchDto {
                        name: branch.name().into(),
                        object_id: branch.object_id().into(),
                        current: branch.is_current(),
                        upstream: branch.upstream().map(Into::into),
                    })
                    .collect()
            })
            .map_err(GitRuntimeError::Service)
    }

    pub(super) fn recent_commits(&self) -> Result<Vec<GitCommitSummaryDto>, GitRuntimeError> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        self.service
            .recent_commits()
            .map(|commits| {
                commits
                    .into_iter()
                    .map(|commit| GitCommitSummaryDto {
                        object_id: commit.object_id().into(),
                        parent_object_ids: commit.parent_object_ids().into(),
                        timestamp_seconds: commit.timestamp_seconds(),
                        subject: commit.subject().into(),
                    })
                    .collect()
            })
            .map_err(GitRuntimeError::Service)
    }

    pub(super) fn graph(
        &self,
        limit: std::num::NonZeroUsize,
        skip: usize,
    ) -> Result<GitGraphResult, GitRuntimeError> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        self.service
            .graph(limit, skip)
            .map(project_graph)
            .map_err(GitRuntimeError::Service)
    }

    pub(super) fn text_diff(&self) -> Result<GitTextDiffResult, GitRuntimeError> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        let (repository, snapshot) = self
            .service
            .text_diff_snapshot()
            .map_err(GitRuntimeError::Service)?;
        let workspace_prefix = self
            .service
            .workspace_root()
            .strip_prefix(repository.worktree_root())
            .map_err(|_| GitRuntimeError::Boundary)?;
        let diffs = snapshot
            .diffs()
            .iter()
            .map(|diff| {
                let path = diff
                    .path()
                    .strip_prefix(workspace_prefix)
                    .map_err(|_| GitRuntimeError::Boundary)?;
                let statistics = diff.statistics();
                Ok(GitTextDiffDto {
                    path: wire_path(path)?,
                    original: diff.original().into(),
                    modified: diff.modified().into(),
                    additions: statistics.additions(),
                    deletions: statistics.deletions(),
                })
            })
            .collect::<Result<Vec<_>, GitRuntimeError>>()?;
        let statistics = snapshot.statistics();
        let repository_snapshot = snapshot.repository().clone();
        let status = self.accept(repository, repository_snapshot)?;
        Ok(GitTextDiffResult {
            status,
            diffs,
            statistics: GitDiffStatisticsDto {
                files: statistics.files(),
                additions: statistics.additions(),
                deletions: statistics.deletions(),
            },
        })
    }

    pub(super) fn switch_branch(&self, name: &str) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_paths(|service| service.switch_branch(name))
    }

    pub(super) fn stage(&self, paths: Vec<PathBuf>) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_paths(|service| service.stage(paths))
    }

    pub(super) fn unstage(&self, paths: Vec<PathBuf>) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_paths(|service| service.unstage(paths))
    }

    pub(super) fn discard_worktree(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_paths(|service| service.discard_worktree(paths))
    }

    pub(super) fn commit(&self, message: String) -> Result<GitRuntimeCommit, GitRuntimeError> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        let GitServiceCommit {
            object_id,
            repository,
            snapshot,
        } = self
            .service
            .commit(message)
            .map_err(GitRuntimeError::Service)?;
        Ok(GitRuntimeCommit {
            object_id,
            status: self.accept(repository, snapshot)?,
        })
    }

    pub(super) fn fetch(&self) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_remote(GitService::fetch)
    }

    pub(super) fn pull_fast_forward(&self) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_remote(GitService::pull_fast_forward)
    }

    pub(super) fn push(&self) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_remote(GitService::push)
    }

    pub(super) fn start_watching(self: &Arc<Self>) -> GitWatcher {
        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
        let runtime = Arc::downgrade(self);
        let thread = std::thread::Builder::new()
            .name("zeta-git-watcher".into())
            .spawn(move || watch_git(runtime, shutdown_rx))
            .ok();
        if thread.is_none() {
            return GitWatcher::default();
        }
        GitWatcher {
            shutdown: Some(shutdown),
            thread,
        }
    }

    fn mutate_paths(
        &self,
        operation: impl FnOnce(
            &GitService,
        )
            -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError>,
    ) -> Result<GitStatusResult, GitRuntimeError> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        let (repository, snapshot) = operation(&self.service).map_err(GitRuntimeError::Service)?;
        self.accept(repository, snapshot)
    }

    fn mutate_remote(
        &self,
        operation: fn(
            &GitService,
        ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError>,
    ) -> Result<GitStatusResult, GitRuntimeError> {
        self.mutate_paths(operation)
    }

    fn accept(
        &self,
        repository: GitRepository,
        snapshot: GitRepositorySnapshot,
    ) -> Result<GitStatusResult, GitRuntimeError> {
        let mut projected = project_status(
            self.stream_instance_id.clone(),
            self.service.workspace_root(),
            &repository,
            snapshot,
        )?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
        let unchanged = state.repository.as_ref() == Some(&repository)
            && state.status.as_ref().is_some_and(|current| {
                current.head == projected.head && current.changes == projected.changes
            });
        if unchanged {
            projected.revision = state.revision;
            return Ok(projected);
        }
        state.revision = state
            .revision
            .checked_add(1)
            .expect("Git status revision overflowed");
        projected.revision = state.revision;
        state.repository = Some(repository);
        state.status = Some(projected.clone());
        drop(state);
        self.updates.publish_git_status_changed(projected.clone());
        Ok(projected)
    }

    fn refresh_from_watcher(&self) {
        let _ = self.status();
    }

    fn watched_paths(&self) -> Vec<WatchPath> {
        let mut paths = vec![
            WatchPath {
                path: self.service.workspace().requested_path().to_path_buf(),
                recursive: true,
            },
            WatchPath {
                path: self.service.workspace().canonical_path().to_path_buf(),
                recursive: true,
            },
        ];
        if let Ok(state) = self.state.lock()
            && let Some(repository) = &state.repository
        {
            paths.push(WatchPath {
                path: repository.git_dir().to_path_buf(),
                recursive: true,
            });
            paths.push(WatchPath {
                path: repository.common_dir().to_path_buf(),
                recursive: true,
            });
            let mut ancestor = self.service.workspace_root().parent();
            while let Some(directory) = ancestor {
                if !directory.starts_with(repository.worktree_root()) {
                    break;
                }
                paths.push(WatchPath {
                    path: directory.join(".gitignore"),
                    recursive: false,
                });
                if directory == repository.worktree_root() {
                    break;
                }
                ancestor = directory.parent();
            }
        }
        paths.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.recursive.cmp(&right.recursive))
        });
        paths.dedup();
        paths
    }
}

impl Drop for GitWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn watch_git(
    runtime: std::sync::Weak<GitRuntime>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    let Ok(tokio_runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    else {
        return;
    };
    tokio_runtime.block_on(async move {
        let Some(git_runtime) = runtime.upgrade() else {
            return;
        };
        let backend = if git_runtime.service.workspace().requested_path()
            == git_runtime.service.workspace().canonical_path()
        {
            FileWatcherBackend::Recommended
        } else {
            FileWatcherBackend::Polling {
                interval: ALIASED_PATH_POLL_INTERVAL,
            }
        };
        let Ok(file_watcher) = FileWatcher::new_with_backend(backend) else {
            return;
        };
        let file_watcher = Arc::new(file_watcher);
        let (subscriber, receiver) = file_watcher.add_subscriber();
        let refresh_runtime = Arc::clone(&git_runtime);
        let _ = tokio::task::spawn_blocking(move || refresh_runtime.refresh_from_watcher()).await;
        let mut watched_paths = git_runtime.watched_paths();
        drop(git_runtime);
        let Ok(mut registration) = subscriber.register_paths(watched_paths.clone()) else {
            return;
        };
        let mut receiver = DebouncedWatchReceiver::new(receiver, GIT_WATCH_DEBOUNCE);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                event = receiver.recv() => {
                    if event.is_none() {
                        break;
                    }
                    let Some(git_runtime) = runtime.upgrade() else {
                        break;
                    };
                    let refresh_runtime = Arc::clone(&git_runtime);
                    let _ = tokio::task::spawn_blocking(move || {
                        refresh_runtime.refresh_from_watcher();
                    }).await;
                    let next_paths = git_runtime.watched_paths();
                    if next_paths != watched_paths
                        && let Ok(next_registration) =
                            subscriber.register_paths(next_paths.clone())
                    {
                        registration = next_registration;
                        watched_paths = next_paths;
                    }
                }
            }
        }
        drop(registration);
    });
}

fn project_graph(graph: GitGraph) -> GitGraphResult {
    GitGraphResult {
        commits: graph
            .commits()
            .iter()
            .map(|commit| GitCommitSummaryDto {
                object_id: commit.object_id().into(),
                parent_object_ids: commit.parent_object_ids().into(),
                timestamp_seconds: commit.timestamp_seconds(),
                subject: commit.subject().into(),
            })
            .collect(),
        references: graph
            .references()
            .iter()
            .map(|reference| GitReferenceDto {
                name: reference.name().into(),
                object_id: reference.object_id().into(),
                kind: match reference.kind() {
                    GitReferenceKind::LocalBranch => GitReferenceKindDto::LocalBranch,
                    GitReferenceKind::RemoteBranch => GitReferenceKindDto::RemoteBranch,
                },
                remote_name: reference.remote_name().map(Into::into),
                current: reference.is_current(),
            })
            .collect(),
        remotes: graph
            .remotes()
            .iter()
            .map(|remote| GitRemoteDto {
                name: remote.name().into(),
                identity: remote.identity().map(|identity| GitRepositoryIdentityDto {
                    provider: match identity.provider() {
                        GitRemoteProvider::Github => GitRemoteProviderDto::Github,
                        GitRemoteProvider::Gitlab => GitRemoteProviderDto::Gitlab,
                        GitRemoteProvider::Bitbucket => GitRemoteProviderDto::Bitbucket,
                        GitRemoteProvider::Other => GitRemoteProviderDto::Other,
                    },
                    host: identity.host().into(),
                    owner: identity.owner().into(),
                    repository: identity.repository().into(),
                }),
            })
            .collect(),
        has_more: graph.has_more(),
    }
}

fn project_status(
    stream_instance_id: StreamInstanceId,
    workspace_root: &Path,
    repository: &GitRepository,
    snapshot: GitRepositorySnapshot,
) -> Result<GitStatusResult, GitRuntimeError> {
    let workspace_prefix = workspace_root
        .strip_prefix(repository.worktree_root())
        .map_err(|_| GitRuntimeError::Boundary)?;
    Ok(GitStatusResult {
        stream_instance_id,
        revision: 0,
        workspace_path: wire_path(workspace_prefix)?,
        head: head(snapshot.head()),
        changes: snapshot
            .changes()
            .iter()
            .filter_map(|change| workspace_change(change, workspace_prefix))
            .collect::<Result<_, _>>()?,
    })
}

fn new_stream_instance_id() -> Result<StreamInstanceId, GitRuntimeError> {
    use std::fmt::Write as _;

    let mut random = [0_u8; 16];
    getrandom::getrandom(&mut random)
        .map_err(|_| GitRuntimeError::Service(GitServiceError::Runtime))?;
    let mut value = String::with_capacity("git_".len() + random.len() * 2);
    value.push_str("git_");
    for byte in random {
        write!(value, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(StreamInstanceId::new(value).expect("generated Git stream instance ID is non-empty"))
}

fn head(head: &GitHead) -> GitHeadDto {
    match head {
        GitHead::Branch {
            name,
            object_id,
            upstream,
        } => GitHeadDto::Branch {
            name: name.clone(),
            object_id: object_id.clone(),
            upstream: upstream.as_ref().map(|upstream| GitUpstreamDto {
                name: upstream.name().into(),
                ahead: upstream.ahead(),
                behind: upstream.behind(),
            }),
        },
        GitHead::Detached { object_id } => GitHeadDto::Detached {
            object_id: object_id.clone(),
        },
        GitHead::Unborn { name } => GitHeadDto::Unborn { name: name.clone() },
    }
}

fn workspace_change(
    change: &GitRepositoryChange,
    workspace_prefix: &Path,
) -> Option<Result<GitRepositoryChangeDto, GitRuntimeError>> {
    let path = change.path().strip_prefix(workspace_prefix).ok()?;
    Some((|| {
        Ok(GitRepositoryChangeDto {
            path: wire_path(path)?,
            original_path: change
                .original_path()
                .and_then(|path| path.strip_prefix(workspace_prefix).ok())
                .map(wire_path)
                .transpose()?,
            index_status: change_status(change.index_status()),
            worktree_status: change_status(change.worktree_status()),
            conflicted: change.is_conflicted(),
            submodule: submodule(change.submodule()),
        })
    })())
}

fn change_status(status: GitChangeStatus) -> GitChangeStatusDto {
    match status {
        GitChangeStatus::Unmodified => GitChangeStatusDto::Unmodified,
        GitChangeStatus::Modified => GitChangeStatusDto::Modified,
        GitChangeStatus::Added => GitChangeStatusDto::Added,
        GitChangeStatus::Deleted => GitChangeStatusDto::Deleted,
        GitChangeStatus::Renamed => GitChangeStatusDto::Renamed,
        GitChangeStatus::Copied => GitChangeStatusDto::Copied,
        GitChangeStatus::TypeChanged => GitChangeStatusDto::TypeChanged,
        GitChangeStatus::Unmerged => GitChangeStatusDto::Unmerged,
        GitChangeStatus::Untracked => GitChangeStatusDto::Untracked,
        GitChangeStatus::Ignored => GitChangeStatusDto::Ignored,
    }
}

fn submodule(state: zeta_git::GitSubmoduleState) -> GitSubmoduleStateDto {
    GitSubmoduleStateDto {
        is_submodule: state.is_submodule(),
        commit_changed: state.commit_changed(),
        tracked_changes: state.tracked_changes(),
        untracked_changes: state.untracked_changes(),
    }
}

fn wire_path(path: &Path) -> Result<String, GitRuntimeError> {
    path.to_str()
        .map(|path| path.replace('\\', "/"))
        .ok_or(GitRuntimeError::Boundary)
}

#[cfg(test)]
#[path = "git_runtime_tests.rs"]
mod tests;
