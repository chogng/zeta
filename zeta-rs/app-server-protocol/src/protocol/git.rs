use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeta_protocol::StreamInstanceId;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum GitChangeStatusDto {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Untracked,
    Ignored,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitUpstreamDto {
    pub name: String,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum GitHeadDto {
    Branch {
        name: String,
        #[serde(rename = "objectId")]
        #[ts(rename = "objectId")]
        object_id: String,
        upstream: Option<GitUpstreamDto>,
    },
    Detached {
        #[serde(rename = "objectId")]
        #[ts(rename = "objectId")]
        object_id: String,
    },
    Unborn {
        name: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitSubmoduleStateDto {
    pub is_submodule: bool,
    pub commit_changed: bool,
    pub tracked_changes: bool,
    pub untracked_changes: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryChangeDto {
    pub path: String,
    pub original_path: Option<String>,
    pub index_status: GitChangeStatusDto,
    pub worktree_status: GitChangeStatusDto,
    pub conflicted: bool,
    pub submodule: GitSubmoduleStateDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusResult {
    pub stream_instance_id: StreamInstanceId,
    #[ts(type = "number")]
    pub revision: u64,
    pub workspace_path: String,
    pub head: GitHeadDto,
    pub changes: Vec<GitRepositoryChangeDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusChanged {
    pub status: GitStatusResult,
}

/// One local branch returned by the workspace Git runtime.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchDto {
    pub name: String,
    pub object_id: String,
    pub current: bool,
    pub upstream: Option<String>,
}

impl GitBranchDto {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn is_current(&self) -> bool {
        self.current
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchListResult {
    pub branches: Vec<GitBranchDto>,
}

/// One commit in the bounded history projection used by repository graph views.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitSummaryDto {
    pub object_id: String,
    pub parent_object_ids: Vec<String>,
    #[ts(type = "number")]
    pub timestamp_seconds: i64,
    pub subject: String,
}

/// Bounded recent commit history for the active workspace repository.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitHistoryResult {
    pub commits: Vec<GitCommitSummaryDto>,
}

/// Provider classification for a configured repository remote.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum GitRemoteProviderDto {
    Github,
    Gitlab,
    Bitbucket,
    Other,
}

/// Credential-free repository identity parsed from a configured Git remote.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GitRepositoryIdentityDto {
    pub provider: GitRemoteProviderDto,
    pub host: String,
    pub owner: String,
    pub repository: String,
}

/// One configured Git remote, with raw URLs intentionally omitted from the wire contract.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GitRemoteDto {
    pub name: String,
    pub identity: Option<GitRepositoryIdentityDto>,
}

/// Ref kinds included in a repository graph snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum GitReferenceKindDto {
    LocalBranch,
    RemoteBranch,
}

/// A local or fetched remote-tracking branch ref.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GitReferenceDto {
    pub name: String,
    pub object_id: String,
    pub kind: GitReferenceKindDto,
    pub remote_name: Option<String>,
    pub current: bool,
}

/// One bounded page request for the active workspace repository graph.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GitGraphParams {
    #[schemars(range(min = 1, max = 1000))]
    pub limit: usize,
    pub skip: usize,
}

/// One bounded commit graph page for the active workspace repository.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GitGraphResult {
    pub commits: Vec<GitCommitSummaryDto>,
    pub references: Vec<GitReferenceDto>,
    pub remotes: Vec<GitRemoteDto>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchSwitchParams {
    #[schemars(length(min = 1, max = 1024))]
    pub name: String,
}

/// One bounded UTF-8 text change from `HEAD` to the workspace working tree.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitTextDiffDto {
    pub path: String,
    pub original: String,
    pub modified: String,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffStatisticsDto {
    pub files: usize,
    pub additions: usize,
    pub deletions: usize,
}

/// One authoritative status snapshot plus its bounded workspace text-diff projection.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitTextDiffResult {
    pub status: GitStatusResult,
    pub diffs: Vec<GitTextDiffDto>,
    pub statistics: GitDiffStatisticsDto,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitPathsParams {
    #[schemars(length(min = 1, max = 5000))]
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitParams {
    #[schemars(length(min = 1, max = 65536))]
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitOperationResult {
    pub status: GitStatusResult,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitResult {
    pub object_id: String,
    pub status: GitStatusResult,
}
