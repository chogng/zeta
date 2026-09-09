use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkRunId;

/// Repository identity remains stable across display-name changes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueRepositoryIdentity {
    pub host: String,
    pub node_id: String,
    pub owner: String,
    pub name: String,
}

impl IssueRepositoryIdentity {
    pub fn key(&self) -> String {
        format!("{}:{}", self.host.to_lowercase(), self.node_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueIdentity {
    pub node_id: String,
    pub number: u64,
    pub title: String,
    pub updated_at: String,
    pub material_digest: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueStage {
    Todo,
    Queued,
    InProgress,
    Review,
    Blocked,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueLabels {
    pub todo: String,
    pub queued: String,
    pub in_progress: String,
    pub review: String,
    pub blocked: String,
}

impl Default for IssueLabels {
    fn default() -> Self {
        Self {
            todo: "status:todo".into(),
            queued: "status:queued".into(),
            in_progress: "status:in-progress".into(),
            review: "status:review".into(),
            blocked: "status:blocked".into(),
        }
    }
}

impl IssueLabels {
    pub fn all(&self) -> [&str; 5] {
        [
            &self.todo,
            &self.queued,
            &self.in_progress,
            &self.review,
            &self.blocked,
        ]
    }
    pub fn for_stage(&self, stage: IssueStage) -> Option<&str> {
        match stage {
            IssueStage::Todo => Some(&self.todo),
            IssueStage::Queued => Some(&self.queued),
            IssueStage::InProgress => Some(&self.in_progress),
            IssueStage::Review => Some(&self.review),
            IssueStage::Blocked => Some(&self.blocked),
            IssueStage::Completed | IssueStage::Cancelled => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueBranchPublication {
    Local,
    Linked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueDelivery {
    Branch,
    PullRequest,
    DraftPullRequest,
}

/// Frozen repository rules shared by every worker using its coordinating service.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueWorkflow {
    pub labels: IssueLabels,
    #[serde(default)]
    pub label_colors: BTreeMap<String, String>,
    #[serde(default)]
    pub auto_merge: bool,
    pub assignee: String,
    pub base_branch: Option<String>,
    pub target_branch: Option<String>,
    pub branch_template: String,
    pub publication: IssueBranchPublication,
    pub coordinator_agent: String,
    pub worker_agent: String,
    pub max_parallel: u32,
    pub max_tokens: u64,
    pub validation_commands: Vec<String>,
    pub delivery: IssueDelivery,
    pub close_on_completion: bool,
    pub auto_claim: Option<IssueAutoClaim>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAutoClaim {
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub max_issues: u32,
}

impl Default for IssueWorkflow {
    fn default() -> Self {
        Self {
            labels: IssueLabels::default(),
            label_colors: BTreeMap::new(),
            auto_merge: false,
            assignee: String::new(),
            base_branch: None,
            target_branch: None,
            branch_template: "codex/issue-{number}-{slug}-{attempt}".into(),
            publication: IssueBranchPublication::Linked,
            coordinator_agent: String::new(),
            worker_agent: String::new(),
            max_parallel: 3,
            max_tokens: 100_000,
            validation_commands: Vec::new(),
            delivery: IssueDelivery::PullRequest,
            close_on_completion: true,
            auto_claim: None,
        }
    }
}

impl IssueWorkflow {
    pub fn validate(&self) -> Result<(), String> {
        if self.validation_commands.len() > 16
            || self.validation_commands.iter().any(|command| {
                command.trim().is_empty() || command.len() > 4096 || command.contains('\0')
            })
        {
            return Err("Provide at most 16 nonempty validation commands".into());
        }
        if self.auto_merge && self.delivery != IssueDelivery::PullRequest {
            return Err("Automatic merge requires a non-draft pull request delivery".into());
        }
        if self.label_colors.iter().any(|(name, color)| {
            !self.labels.all().contains(&name.as_str())
                || color.len() != 6
                || !color.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(
                "Stage colors must be six hexadecimal digits for a configured label".into(),
            );
        }
        let labels = self.labels.all();
        if labels.iter().any(|label| {
            label.trim() != *label
                || label.is_empty()
                || label.len() > 50
                || label.chars().any(char::is_control)
        }) || labels
            .iter()
            .map(|label| label.to_lowercase())
            .collect::<BTreeSet<_>>()
            .len()
            != labels.len()
        {
            return Err(
                "Issue stage labels must be distinct, nonempty names of at most 50 bytes".into(),
            );
        }
        if !(1..=16).contains(&self.max_parallel)
            || !(1_000..=10_000_000).contains(&self.max_tokens)
        {
            return Err(
                "Issue workflow requires 1–16 workers and a 1000–10000000 token budget".into(),
            );
        }
        if !self.branch_template.contains("{number}") || !self.branch_template.contains("{attempt}")
        {
            return Err("Issue branch template must contain {number} and {attempt}".into());
        }
        let branch = self
            .branch_template
            .replace("{number}", "1")
            .replace("{slug}", "issue")
            .replace("{attempt}", "a1");
        validate_branch(&branch)?;
        for branch in [&self.base_branch, &self.target_branch]
            .into_iter()
            .flatten()
        {
            validate_branch(branch)?;
        }
        if self.assignee.len() > 100
            || self
                .assignee
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err("Invalid issue assignee".into());
        }
        for agent in [&self.coordinator_agent, &self.worker_agent] {
            if agent.len() > 255 || agent.chars().any(char::is_control) {
                return Err("Invalid issue Agent definition".into());
            }
        }
        if let Some(auto) = &self.auto_claim {
            if auto.labels.is_empty()
                || auto.labels.iter().any(|label| label.trim().is_empty())
                || auto
                    .assignee
                    .as_ref()
                    .is_some_and(|assignee| assignee.trim().is_empty())
                || self.assignee.is_empty()
                || auto
                    .assignee
                    .as_ref()
                    .is_some_and(|assignee| assignee != &self.assignee)
                || !(1..=20).contains(&auto.max_issues)
            {
                return Err(
                    "Automatic claiming requires labels, an assignee, and a 1–20 issue limit"
                        .into(),
                );
            }
        }
        Ok(())
    }

    pub fn branch_name(&self, issue: &IssueIdentity, attempt: &str) -> Result<String, String> {
        let slug = issue
            .title
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let slug = if slug.is_empty() {
            "issue"
        } else {
            &slug[..slug.len().min(48)]
        };
        let branch = self
            .branch_template
            .replace("{number}", &issue.number.to_string())
            .replace("{slug}", slug)
            .replace("{attempt}", attempt);
        validate_branch(&branch)?;
        Ok(branch)
    }
}

fn validate_branch(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 200
        || value.starts_with(['-', '/'])
        || value.ends_with(['/', '.'])
        || value.contains("..")
        || value.contains("@{")
        || value.contains("//")
        || value
            .split('/')
            .any(|part| part.starts_with('.') || part.ends_with(".lock"))
        || value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "~^:?*[\\{}".contains(c))
    {
        return Err("Invalid issue branch name or template".into());
    }
    Ok(())
}

/// One accepted work item; dependencies reference item ids rather than mutable Agent messages.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueWorkItem {
    pub id: String,
    pub issues: Vec<IssueIdentity>,
    pub objective: String,
    pub acceptance_conditions: Vec<String>,
    pub scope: crate::WorkScopeClaim,
    pub dependencies: BTreeSet<String>,
    pub agent: String,
}

impl IssueWorkItem {
    /// Unknown scopes serialize; declared shared APIs and resources cannot have concurrent writers.
    pub fn conflicts_with(&self, other: &Self) -> bool {
        let empty = |scope: &crate::WorkScopeClaim| {
            scope.paths.is_empty()
                && scope.components.is_empty()
                && scope.contracts.is_empty()
                && scope.resources.is_empty()
        };
        empty(&self.scope)
            || empty(&other.scope)
            || !self.scope.components.is_disjoint(&other.scope.components)
            || !self.scope.contracts.is_disjoint(&other.scope.contracts)
            || !self.scope.resources.is_disjoint(&other.scope.resources)
            || self.scope.paths.iter().any(|left| {
                other.scope.paths.iter().any(|right| {
                    let left = left.split('*').next().unwrap_or("").trim_end_matches('/');
                    let right = right.split('*').next().unwrap_or("").trim_end_matches('/');
                    left.is_empty()
                        || right.is_empty()
                        || left == right
                        || left.starts_with(&format!("{right}/"))
                        || right.starts_with(&format!("{left}/"))
                })
            })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignmentPlan {
    pub repository: IssueRepositoryIdentity,
    pub workflow: IssueWorkflow,
    pub config_revision: u64,
    pub model: Option<zeta_protocol::ModelRef>,
    #[serde(default)]
    pub planning_tokens: u64,
    pub base_commit: String,
    pub target_branch: String,
    pub items: Vec<IssueWorkItem>,
}

impl IssueAssignmentPlan {
    pub fn validate(&self) -> Result<(), String> {
        self.workflow.validate()?;
        if self.planning_tokens >= self.workflow.max_tokens {
            return Err("Planning exhausted the batch token budget".into());
        }
        if self.repository.node_id.is_empty()
            || self.repository.host.is_empty()
            || self.items.is_empty()
            || self.items.len() > 20
        {
            return Err("Assignment requires a repository and 1–20 work items".into());
        }
        validate_branch(&self.target_branch)?;
        if !(40..=64).contains(&self.base_commit.len())
            || !self.base_commit.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Assignment requires a fixed Git commit".into());
        }
        let ids = self
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<BTreeSet<_>>();
        if ids.len() != self.items.len() || ids.contains("") {
            return Err("Work item ids must be distinct".into());
        }
        let mut issues = BTreeSet::new();
        for item in &self.items {
            if item.issues.is_empty()
                || item.objective.trim().is_empty()
                || item.acceptance_conditions.is_empty()
                || item
                    .acceptance_conditions
                    .iter()
                    .any(|condition| condition.trim().is_empty())
            {
                return Err(
                    "Each work item requires issues, an objective and acceptance conditions".into(),
                );
            }
            for issue in &item.issues {
                if issue.node_id.is_empty()
                    || issue.number == 0
                    || !issues.insert(issue.node_id.as_str())
                {
                    return Err("Each issue must belong to exactly one work item".into());
                }
            }
            if item
                .dependencies
                .iter()
                .any(|id| id == &item.id || !ids.contains(id.as_str()))
            {
                return Err("Invalid issue dependency".into());
            }
        }
        if issues.len() > 20 {
            return Err("At most 20 issues may be assigned in one batch".into());
        }
        let mut remaining = self
            .items
            .iter()
            .map(|item| (item.id.clone(), item.dependencies.clone()))
            .collect::<BTreeMap<_, _>>();
        while !remaining.is_empty() {
            let ready = remaining
                .iter()
                .filter(|(_, dependencies)| dependencies.is_empty())
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            if ready.is_empty() {
                return Err("Issue dependencies contain a cycle".into());
            }
            for id in ready {
                remaining.remove(&id);
                for dependencies in remaining.values_mut() {
                    dependencies.remove(&id);
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueOwnership {
    Unclaimed,
    Held,
    Releasing,
    Transferring,
    Released,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueSyncState {
    Pending,
    Synced,
    Conflict,
}

/// Durable Issue membership and provisioning receipts; execution facts belong to WorkAttempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueAssignment {
    pub id: String,
    pub config_revision: u64,
    pub batch_id: String,
    pub repository: IssueRepositoryIdentity,
    pub item: IssueWorkItem,
    pub workflow: IssueWorkflow,
    #[serde(default)]
    pub agent_role: Option<zeta_protocol::AgentRoleSnapshot>,
    #[serde(default)]
    pub agent_tools: Vec<zeta_protocol::ToolName>,
    pub model: Option<zeta_protocol::ModelRef>,
    #[serde(default)]
    pub planning_tokens: u64,
    pub base_commit: String,
    pub target_branch: String,
    pub branch: String,
    pub delivery: Option<IssueDeliveryReceipt>,
    pub owner: String,
    pub pending_owner: Option<String>,
    pub auto_start: bool,
    pub revision: u64,
    pub epoch: u64,
    pub ownership: IssueOwnership,
    pub thread_id: Option<ThreadId>,
    pub work_run_id: Option<WorkRunId>,
    pub attempt_id: Option<WorkAttemptId>,
    pub lease_until: Option<u64>,
    pub sync_state: IssueSyncState,
    pub attempted_stages: Vec<IssueStage>,
    pub desired_stage: IssueStage,
    #[serde(default)]
    pub execution_error: Option<String>,
    pub paused: bool,
    pub synced_stage: Option<IssueStage>,
    pub synced_labels: BTreeMap<String, Vec<String>>,
    pub linked_branch_id: Option<String>,
    pub detail: String,
    pub updated_at: u64,
}

impl IssueAssignment {
    pub fn execution_budget(&self) -> u64 {
        self.workflow
            .max_tokens
            .saturating_sub(self.planning_tokens)
    }

    pub fn check_writer(&self, epoch: u64, now: u64) -> Result<(), String> {
        if self.ownership != IssueOwnership::Held
            || self.epoch != epoch
            || self.lease_until.is_none_or(|deadline| now >= deadline)
        {
            return Err("Issue execution lease is absent, expired or superseded".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "issue_assignment_tests.rs"]
mod tests;

/// Exact tested delivery candidate; remote publication resumes from this immutable receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueDeliveryReceipt {
    pub target_head: String,
    pub tree: String,
    pub commit: String,
    pub verification_key: String,
    pub pull_request_number: Option<u64>,
    pub pull_request_url: Option<String>,
}
