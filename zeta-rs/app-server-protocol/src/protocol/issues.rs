use crate::protocol::common::CommandId;
use crate::protocol::common::SessionId;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueConfigDto {
    pub recommend_merge: bool,
    pub auto_refresh_minutes: u32,
    pub analysis_model: Option<crate::protocol::config::ModelRefDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueConfigureParams {
    pub command_id: CommandId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub config: IssueConfigDto,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum IssueStartPoint {
    CurrentBranch,
    Main,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueTaskCreateParams {
    pub command_id: CommandId,
    pub repository: IssueRepository,
    #[ts(type = "number[]")]
    pub numbers: Vec<u64>,
    pub start: IssueStartPoint,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueTaskReadParams {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueTaskResult {
    pub task: Option<IssueTask>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueTask {
    pub pending_input: bool,
    pub session_id: SessionId,
    pub repository: IssueRepository,
    pub issues: Vec<IssueReadResult>,
    pub start_commit: String,
    pub branch: String,
    pub target_branch: String,
    #[ts(type = "number")]
    pub read_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueRepository {
    pub host: String,
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueSummary {
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    #[ts(type = "number")]
    pub number: u64,
    pub title: String,
    pub url: String,
    pub updated_at: String,
    pub state: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum IssueListMode {
    Cached,
    Auto,
    Refresh,
    ClearCache,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueListParams {
    pub state: IssueState,
    pub page: u32,
    pub query: String,
    pub mode: IssueListMode,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueListResult {
    pub repository: IssueRepository,
    pub issues: Vec<IssueSummary>,
    pub next_page: Option<u32>,
    pub cached: bool,
    #[ts(type = "number")]
    pub fetched_at: u64,
    pub refresh_after_seconds: Option<u32>,
    pub notice: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueReadParams {
    pub repository: IssueRepository,
    #[ts(type = "number")]
    pub number: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueReadResult {
    pub issue: IssueSummary,
    pub body: String,
    pub comments: Vec<IssueComment>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssueComment {
    pub body: String,
    pub url: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum IssuePrMode {
    Ordinary,
    Draft,
    Merge,
    Squash,
    Rebase,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssuePrPreview {
    pub session_id: SessionId,
    pub title: String,
    pub body: String,
    pub branch: String,
    pub target_branch: String,
    pub start_commit: String,
    pub target_commit: String,
    pub expected_tree: String,
    pub files: Vec<String>,
    pub modes: Vec<IssuePrMode>,
    pub pull_request: Option<IssuePrStatus>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssuePrCreateParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    pub expected_tree: String,
    pub mode: IssuePrMode,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IssuePrStatus {
    pub url: String,
    pub state: String,
    pub draft: bool,
    pub merged: bool,
    pub automatic_merge: Option<bool>,
    pub refresh_error: Option<String>,
    pub checks: String,
    pub automatic_merge_error: Option<String>,
}
