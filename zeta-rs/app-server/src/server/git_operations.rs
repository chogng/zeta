use super::{AppServer, RpcError, decode, result};
use crate::git_service::GitServiceError;
use crate::server::git_runtime::GitRuntimeError;
use serde_json::Value;
use std::path::PathBuf;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::git::{
    GitBranchListResult, GitBranchSwitchParams, GitCommitParams,
    GitCommitResult as GitCommitResultDto, GitHistoryResult, GitOperationResult, GitPathsParams,
};
use zeta_git::GitError;

impl AppServer {
    pub(super) fn git_status(&self) -> Result<Value, RpcError> {
        result(&self.git_runtime_service()?.status().map_err(git_error)?)
    }

    pub(super) fn git_text_diff(&self) -> Result<Value, RpcError> {
        result(&self.git_runtime_service()?.text_diff().map_err(git_error)?)
    }

    pub(super) fn git_branch_list(&self) -> Result<Value, RpcError> {
        let branches = self
            .git_runtime_service()?
            .local_branches()
            .map_err(git_error)?;
        result(&GitBranchListResult { branches })
    }

    pub(super) fn git_history(&self) -> Result<Value, RpcError> {
        let commits = self
            .git_runtime_service()?
            .recent_commits()
            .map_err(git_error)?;
        result(&GitHistoryResult { commits })
    }

    pub(super) fn git_graph(&self) -> Result<Value, RpcError> {
        result(&self.git_runtime_service()?.graph().map_err(git_error)?)
    }

    pub(super) fn git_branch_switch(&self, params: &Value) -> Result<Value, RpcError> {
        let params: GitBranchSwitchParams = decode(params)?;
        if params.name.trim().is_empty() || params.name.len() > 1024 || params.name.contains('\0') {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let status = self
            .git_runtime_service()?
            .switch_branch(&params.name)
            .map_err(git_error)?;
        result(&GitOperationResult { status })
    }

    pub(super) fn git_stage(&self, params: &Value) -> Result<Value, RpcError> {
        let params: GitPathsParams = decode(params)?;
        let status = self
            .git_runtime_service()?
            .stage(workspace_paths(params.paths)?)
            .map_err(git_error)?;
        result(&GitOperationResult { status })
    }

    pub(super) fn git_unstage(&self, params: &Value) -> Result<Value, RpcError> {
        let params: GitPathsParams = decode(params)?;
        let status = self
            .git_runtime_service()?
            .unstage(workspace_paths(params.paths)?)
            .map_err(git_error)?;
        result(&GitOperationResult { status })
    }

    pub(super) fn git_discard_worktree(&self, params: &Value) -> Result<Value, RpcError> {
        let params: GitPathsParams = decode(params)?;
        let status = self
            .git_runtime_service()?
            .discard_worktree(workspace_paths(params.paths)?)
            .map_err(git_error)?;
        result(&GitOperationResult { status })
    }

    pub(super) fn git_commit(&self, params: &Value) -> Result<Value, RpcError> {
        let params: GitCommitParams = decode(params)?;
        if params.message.trim().is_empty()
            || params.message.len() > 65_536
            || params.message.contains('\0')
        {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let committed = self
            .git_runtime_service()?
            .commit(params.message)
            .map_err(git_error)?;
        result(&GitCommitResultDto {
            object_id: committed.object_id,
            status: committed.status,
        })
    }

    pub(super) fn git_fetch(&self) -> Result<Value, RpcError> {
        let status = self.git_runtime_service()?.fetch().map_err(git_error)?;
        result(&GitOperationResult { status })
    }

    pub(super) fn git_pull(&self) -> Result<Value, RpcError> {
        let status = self
            .git_runtime_service()?
            .pull_fast_forward()
            .map_err(git_error)?;
        result(&GitOperationResult { status })
    }

    pub(super) fn git_push(&self) -> Result<Value, RpcError> {
        let status = self.git_runtime_service()?.push().map_err(git_error)?;
        result(&GitOperationResult { status })
    }
}

fn workspace_paths(paths: Vec<String>) -> Result<Vec<PathBuf>, RpcError> {
    if paths.is_empty() || paths.len() > 5_000 {
        return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
    }
    let paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    if paths.iter().any(|path| {
        path.as_os_str().is_empty()
            || path.as_os_str().to_string_lossy().contains('\0')
            || path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
    }) {
        return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
    }
    Ok(paths)
}

fn git_error(error: GitRuntimeError) -> RpcError {
    match error {
        GitRuntimeError::Boundary
        | GitRuntimeError::Service(GitServiceError::Boundary)
        | GitRuntimeError::Service(GitServiceError::BranchNotFound) => {
            RpcError::new(-32061, AppServerErrorName::GitOperationFailed)
        }
        GitRuntimeError::Service(GitServiceError::Git(GitError::NotAWorkingTree { .. })) => {
            RpcError::new(-32062, AppServerErrorName::GitNotRepository)
        }
        GitRuntimeError::Service(GitServiceError::Git(_)) => {
            RpcError::new(-32061, AppServerErrorName::GitOperationFailed)
        }
        GitRuntimeError::Service(GitServiceError::Runtime) => {
            RpcError::new(-32000, AppServerErrorName::ServerOverloaded)
        }
        GitRuntimeError::Service(GitServiceError::Trust) => {
            RpcError::new(-32060, AppServerErrorName::GitUnavailable)
        }
    }
}
