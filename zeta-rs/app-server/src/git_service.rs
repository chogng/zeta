use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::runtime::Runtime;
use zeta_git::{
    GitClient, GitCommitRequest, GitError, GitPathspecSet, GitRepository, GitRepositorySnapshot,
};

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
    workspace_root: PathBuf,
    client: GitClient,
    runtime: Mutex<Runtime>,
}

impl GitService {
    pub(crate) fn new(workspace_root: PathBuf) -> Result<Self, GitServiceError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| GitServiceError::Runtime)?;
        Ok(Self {
            workspace_root,
            client: GitClient::system(),
            runtime: Mutex::new(runtime),
        })
    }

    pub(crate) fn snapshot(
        &self,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        let runtime = self.runtime.lock().map_err(|_| GitServiceError::Runtime)?;
        runtime.block_on(async {
            let repository = self
                .client
                .open_repository(&self.workspace_root)
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
        &self.workspace_root
    }

    pub(crate) fn stage(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<(GitRepository, GitRepositorySnapshot), GitServiceError> {
        self.mutate_paths(GitPathMutation::Stage, paths)
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
            .open_repository(&self.workspace_root)
            .await
            .map_err(GitServiceError::Git)
    }

    fn repository_paths(
        &self,
        repository: &GitRepository,
        paths: Vec<PathBuf>,
    ) -> Result<GitPathspecSet, GitServiceError> {
        let workspace_prefix = self
            .workspace_root
            .strip_prefix(repository.worktree_root())
            .map_err(|_| GitServiceError::Boundary)?;
        GitPathspecSet::new(
            paths
                .into_iter()
                .map(|path| workspace_prefix.join(path))
                .collect(),
        )
        .map_err(GitServiceError::Git)
    }
}

#[derive(Debug)]
pub(crate) enum GitServiceError {
    Boundary,
    Git(GitError),
    Runtime,
}
