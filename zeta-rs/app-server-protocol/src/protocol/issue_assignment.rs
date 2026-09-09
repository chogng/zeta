use super::work_run_model::WorkScopeClaimDto;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use ts_rs::TS;
use zeta_protocol::CommandId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkRunId;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueRepositoryIdentityDto {
    pub host: String,
    pub node_id: String,
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueIdentityDto {
    pub node_id: String,
    #[ts(type = "number")]
    pub number: u64,
    pub title: String,
    pub updated_at: String,
    pub material_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssueStageDto {
    Todo,
    Queued,
    InProgress,
    Review,
    Blocked,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueLabelsDto {
    pub todo: String,
    pub queued: String,
    pub in_progress: String,
    pub review: String,
    pub blocked: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssueBranchPublicationDto {
    Local,
    Linked,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssueDeliveryDto {
    Branch,
    PullRequest,
    DraftPullRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueWorkflowDto {
    pub labels: IssueLabelsDto,
    #[serde(default)]
    pub label_colors: BTreeMap<String, String>,
    #[serde(default)]
    pub auto_merge: bool,
    pub assignee: String,
    pub base_branch: Option<String>,
    pub target_branch: Option<String>,
    pub branch_template: String,
    pub publication: IssueBranchPublicationDto,
    pub coordinator_agent: String,
    pub worker_agent: String,
    pub max_parallel: u32,
    #[ts(type = "number")]
    pub max_tokens: u64,
    pub validation_commands: Vec<String>,
    pub delivery: IssueDeliveryDto,
    pub close_on_completion: bool,
    pub auto_claim: Option<IssueAutoClaimDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAutoClaimDto {
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub max_issues: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueWorkItemDto {
    pub id: String,
    pub issues: Vec<IssueIdentityDto>,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub scope: WorkScopeClaimDto,
    pub dependencies: BTreeSet<String>,
    pub agent: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentPlanDto {
    pub repository: IssueRepositoryIdentityDto,
    pub workflow: IssueWorkflowDto,
    #[ts(type = "number")]
    pub config_revision: u64,
    pub model: Option<super::config::ModelRefDto>,
    #[serde(default)]
    #[ts(type = "number")]
    pub planning_tokens: u64,
    pub base_commit: String,
    pub target_branch: String,
    pub items: Vec<IssueWorkItemDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssueOwnershipDto {
    Unclaimed,
    Held,
    Releasing,
    Transferring,
    Released,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssueSyncStateDto {
    Pending,
    Synced,
    Conflict,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentDto {
    pub id: String,
    #[ts(type = "number")]
    pub config_revision: u64,
    pub batch_id: String,
    pub repository: IssueRepositoryIdentityDto,
    pub item: IssueWorkItemDto,
    pub workflow: IssueWorkflowDto,
    #[serde(default)]
    pub agent_role: Option<zeta_protocol::AgentRoleSnapshot>,
    #[serde(default)]
    pub agent_tools: Vec<zeta_protocol::ToolName>,
    pub model: Option<super::config::ModelRefDto>,
    #[serde(default)]
    #[ts(type = "number")]
    pub planning_tokens: u64,
    pub base_commit: String,
    pub target_branch: String,
    pub branch: String,
    pub delivery: Option<IssueDeliveryReceiptDto>,
    pub owner: String,
    pub pending_owner: Option<String>,
    pub auto_start: bool,
    #[ts(type = "number")]
    pub revision: u64,
    #[ts(type = "number")]
    pub epoch: u64,
    pub ownership: IssueOwnershipDto,
    pub thread_id: Option<ThreadId>,
    pub work_run_id: Option<WorkRunId>,
    pub attempt_id: Option<WorkAttemptId>,
    #[ts(type = "number | null")]
    pub lease_until: Option<u64>,
    pub sync_state: IssueSyncStateDto,
    pub attempted_stages: Vec<IssueStageDto>,
    pub desired_stage: IssueStageDto,
    #[serde(default)]
    pub execution_error: Option<String>,
    pub paused: bool,
    pub synced_stage: Option<IssueStageDto>,
    pub synced_labels: BTreeMap<String, Vec<String>>,
    pub linked_branch_id: Option<String>,
    pub detail: String,
    #[ts(type = "number")]
    pub updated_at: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueWorkflowReadResult {
    pub repository: IssueRepositoryIdentityDto,
    #[ts(type = "number")]
    pub config_revision: u64,
    pub workflow: IssueWorkflowDto,
    pub default_branch: String,
    pub labels: Vec<IssueLabelDto>,
    pub assignees: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueLabelDto {
    pub name: String,
    pub color: String,
    pub node_id: String,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueWorkflowConfigureParams {
    pub command_id: CommandId,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub repository: IssueRepositoryIdentityDto,
    pub workflow: IssueWorkflowDto,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueLabelCreateParams {
    pub repository: IssueRepositoryIdentityDto,
    pub name: String,
    pub color: String,
    pub expected_color: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssuePlanMode {
    Branch,
    Combined,
    Distributed,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssuePlanParams {
    #[ts(type = "number[]")]
    pub numbers: Vec<u64>,
    pub mode: IssuePlanMode,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssuePlanResult {
    pub plan: IssueAssignmentPlanDto,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssueAssignmentStartAction {
    CreateBranch,
    Claim,
    Execute,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentStartParams {
    pub command_id: CommandId,
    pub plan: IssueAssignmentPlanDto,
    pub action: IssueAssignmentStartAction,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentView {
    pub assignment: IssueAssignmentDto,
    pub stage: IssueStageDto,
    pub health: String,
    pub branch_url: Option<String>,
    pub pull_request_url: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentsResult {
    pub assignments: Vec<IssueAssignmentView>,
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IssueAssignmentAction {
    Pause,
    Resume,
    Release,
    Cancel,
    RetrySync,
    Verify,
    Deliver,
    Transfer { assignee: String },
}
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentActionParams {
    pub command_id: CommandId,
    pub assignment_id: String,
    #[ts(type = "number")]
    pub expected_epoch: u64,
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub action: IssueAssignmentAction,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueDeliveryReceiptDto {
    pub target_head: String,
    pub tree: String,
    pub commit: String,
    pub verification_key: String,
    #[ts(type = "number | null")]
    pub pull_request_number: Option<u64>,
    pub pull_request_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentNotice {
    pub assignment_id: String,
    pub message: String,
}
