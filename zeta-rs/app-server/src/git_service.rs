use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::runtime::Runtime;
use zeta_git::{
    GitBranch, GitClient, GitCommitRequest, GitCommitSummary, GitError, GitPathspecSet,
    GitRepository, GitRepositorySnapshot, GitTextDiffLimits, GitTextDiffSnapshot,
};
use zeta_workspace::{TrustedWorkspace, WorkspaceCapability, WorkspaceRoot};

const MAX_TEXT_DIFF_FILE_BYTES: usize = 2 * 1024 * 1024;
const RECENT_COMMIT_LIMIT: NonZeroUsize = NonZeroUsize::new(50).expect("history limit is non-zero");

pub(crate) struct GitServiceCommit {
    pub(crate) object_id: String,
    pub(crate) repository: GitRepository,
    pub(crate) snapshot: GitRepositorySnapshot,
}

#[derive(Clone, Copy)]
enum GitPathMutation {
    Stage,
    Unstage,
    DiscardWorktree,
}

#[derive(Clone, Copy)]
enum GitRemoteMutation {
    Fetch,
    PullFastForward,
    Push,
}

/// Workspace-scoped owner of the async Git runtime used by synchronous RPC dispatch.
pub(crate) struct GitService {
    workspace: TrustedWorkspace,
    client: GitClient,
    runtime: Mutex<Runtime>,
}

impl GitService {
    pub(crate) fn new(workspace: TrustedWorkspace) -> Result<Self, GitServiceError> {
        if !matches!(
            workspace.capability(),
            WorkspaceCapability::InspectRepository | WorkspaceCapability::MutateRepository
        ) {
            return Err(GitServiceError::Trust);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| GitServiceError::Runtime)?;
        Ok(Self {
            workspace,
            client: GitClient::system(),
            runtime: Mutex::new(runtime),
        })
    }

    pub(crate) fn snapshot(
        &self,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.ensure_readable()?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self
                .client
                .open_repository(self.workspace.root().canonical_path())
                .await
                .map_err(GitServiceError::Git)?;
            let snapshot = self
                .client
                .snapshot(&repository)
                .await
                .map_err(GitServiceError::Git)?;
            Ok((repository, snapshot))
        })
    }

    pub(crate) fn workspace_root(&self) -> &Path {
        self.workspace.root().canonical_path()
    }

    pub(crate) fn workspace(&self) -> &WorkspaceRoot {
        self.workspace.root()
    }

    pub(crate) fn stage(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.mutate_paths(GitPathMutation::Stage, paths)
    }

    pub(crate) fn local_branches(&self) -> Result<Vec<GitBranch>, GitServiceError> {
        self.ensure_readable()?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self.open_repository().await?;
            self.client
                .local_branches(&repository)
                .await
                .map_err(GitServiceError::Git)
        })
    }

    pub(crate) fn recent_commits(&self) -> Result<Vec<GitCommitSummary>, GitServiceError> {
        self.ensure_readable()?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self.open_repository().await?;
            self.client
                .recent_commits(&repository, RECENT_COMMIT_LIMIT)
                .await
                .map_err(GitServiceError::Git)
        })
    }

    pub(crate) fn text_diff_snapshot(
        &self,
    ) -> Result<(GitRepository, GitTextDiffSnapshot), GitServiceError> {
        self.ensure_readable()?;
        let limits =
            GitTextDiffLimits::new(MAX_TEXT_DIFF_FILE_BYTES).map_err(GitServiceError::Git)?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self.open_repository().await?;
            let workspace_prefix = self
                .workspace
                .root()
                .relative_to_existing_ancestor(repository.worktree_root())
                .map_err(|_| GitServiceError::Boundary)?;
            let snapshot = self
                .client
                .text_diff_snapshot_under(&repository, &workspace_prefix, limits)
                .await
                .map_err(GitServiceError::Git)?;
            Ok((repository, snapshot))
        })
    }

    pub(crate) fn switch_branch(
        &self,
        name: &str,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.ensure_mutable()?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self.open_repository().await?;
            let branches = self
                .client
                .local_branches(&repository)
                .await
                .map_err(GitServiceError::Git)?;
            let branch = branches
                .iter()
                .find(|branch| branch.name() == name)
                .ok_or(GitServiceError::BranchNotFound)?;
            self.client
                .switch_branch(&repository, branch)
                .await
                .map_err(GitServiceError::Git)?;
            let snapshot = self
                .client
                .snapshot(&repository)
                .await
                .map_err(GitServiceError::Git)?;
            Ok((repository, snapshot))
        })
    }

    pub(crate) fn unstage(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.mutate_paths(GitPathMutation::Unstage, paths)
    }

    pub(crate) fn discard_worktree(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.mutate_paths(GitPathMutation::DiscardWorktree, paths)
    }

    pub(crate) fn commit(&self, message: String) -> Result<GitServiceCommit, GitServiceError> {
        self.ensure_mutable()?;
        let request = GitCommitRequest::new(message).map_err(GitServiceError::Git)?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self.open_repository().await?;
            let committed = self
                .client
                .commit(&repository, &request)
                .await
                .map_err(GitServiceError::Git)?;
            let snapshot = self
                .client
                .snapshot(&repository)
                .await
                .map_err(GitServiceError::Git)?;
            Ok(GitServiceCommit {
                object_id: committed.object_id().into(),
                repository,
                snapshot,
            })
        })
    }

    pub(crate) fn fetch(&self) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.mutate_remote(GitRemoteMutation::Fetch)
    }

    pub(crate) fn pull_fast_forward(
        &self,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.mutate_remote(GitRemoteMutation::PullFastForward)
    }

    pub(crate) fn push(&self) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.mutate_remote(GitRemoteMutation::Push)
    }

    fn mutate_paths(
        &self,
        operation: GitPathMutation,
        paths: Vec<PathBuf>,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.ensure_mutable()?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self.open_repository().await?;
            let paths = self.repository_paths(&repository, paths)?;
            match operation {
                GitPathMutation::Stage => self.client.stage(&repository, &paths).await,
                GitPathMutation::Unstage => self.client.unstage(&repository, &paths).await,
                GitPathMutation::DiscardWorktree => {
                    self.client.discard_worktree(&repository, &paths).await
                }
            }
            .map_err(GitServiceError::Git)?;
            let snapshot = self
                .client
                .snapshot(&repository)
                .await
                .map_err(GitServiceError::Git)?;
            Ok((repository, snapshot))
        })
    }

    fn mutate_remote(
        &self,
        operation: GitRemoteMutation,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.ensure_mutable()?;
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self.open_repository().await?;
            match operation {
                GitRemoteMutation::Fetch => self.client.fetch(&repository).await,
                GitRemoteMutation::PullFastForward => {
                    self.client.pull_fast_forward(&repository).await
                }
                GitRemoteMutation::Push => self.client.push(&repository).await,
            }
            .map_err(GitServiceError::Git)?;
            let snapshot = self
                .client
                .snapshot(&repository)
                .await
                .map_err(GitServiceError::Git)?;
            Ok((repository, snapshot))
        })
    }

    async fn open_repository(&self) -> Result<GitRepository, GitServiceError> {
        self.client
            .open_repository(self.workspace.root().canonical_path())
            .await
            .map_err(GitServiceError::Git)
    }

    fn repository_paths(
        &self,
        repository: &GitRepository,
        paths: Vec<PathBuf>,
    ) -> Result<GitPathspecSet, GitServiceError> {
        let workspace_prefix = self
            .workspace
            .root()
            .relative_to_existing_ancestor(repository.worktree_root())
            .map_err(|_| GitServiceError::Boundary)?;
        GitPathspecSet::new(
            paths
                .into_iter()
                .map(|path| workspace_prefix.join(path))
                .collect(),
        )
        .map_err(GitServiceError::Git)
    }

    fn ensure_readable(&self) -> Result<(), GitServiceError> {
        if !matches!(
            self.workspace.capability(),
            WorkspaceCapability::InspectRepository | WorkspaceCapability::MutateRepository
        ) {
            return Err(GitServiceError::Trust);
        }
        self.workspace
            .ensure_active()
            .map_err(|_| GitServiceError::Trust)
    }

    fn ensure_mutable(&self) -> Result<(), GitServiceError> {
        if self.workspace.capability() != WorkspaceCapability::MutateRepository {
            return Err(GitServiceError::Trust);
        }
        self.workspace
            .ensure_active()
            .map_err(|_| GitServiceError::Trust)
    }
}

#[derive(Debug)]
pub(crate) enum GitServiceError {
    BranchNotFound,
    Boundary,
    Git(GitError),
    Runtime,
    Trust,
}
