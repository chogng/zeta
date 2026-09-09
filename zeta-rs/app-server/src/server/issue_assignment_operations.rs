use super::AppServer;
use super::RpcError;
use super::decode;
use super::issue_assignment::convert;
use super::issue_assignment::now;
use super::issue_operations::issue_error;
use super::result;
use serde_json::Value;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::issue_assignment::IssueAssignmentAction;
use zeta_app_server_protocol::protocol::issue_assignment::IssueAssignmentActionParams;
use zeta_app_server_protocol::protocol::issue_assignment::IssueAssignmentStartAction;
use zeta_app_server_protocol::protocol::issue_assignment::IssueAssignmentStartParams;
use zeta_app_server_protocol::protocol::issue_assignment::IssueAssignmentView;
use zeta_app_server_protocol::protocol::issue_assignment::IssueAssignmentsResult;
use zeta_state::IssueAssignmentCommand;
use zeta_work_coordination::IssueAssignment;
use zeta_work_coordination::IssueAssignmentPlan;
use zeta_work_coordination::IssueBranchPublication;
use zeta_work_coordination::IssueOwnership;
use zeta_work_coordination::IssueStage;
use zeta_work_coordination::IssueSyncState;

impl AppServer {
    pub(super) fn issue_assignment_store(
        &self,
    ) -> Result<&zeta_state::SqliteIssueAssignmentStore, RpcError> {
        self.issue_assignments
            .as_deref()
            .ok_or_else(|| issue_error("Issue assignment storage unavailable".into()))
    }

    pub(super) fn issue_assignments_list(&self) -> Result<Value, RpcError> {
        let (_, repository, _) = self.cached_issue_repository_identity()?;
        let assignments = self
            .issue_assignment_store()?
            .list(&repository.key())
            .map_err(issue_error)?;
        self.issue_assignment_views(assignments)
    }

    pub(super) fn issue_assignment_start(&self, value: &Value) -> Result<Value, RpcError> {
        let params: IssueAssignmentStartParams = decode(value)?;
        let plan: IssueAssignmentPlan = convert(&params.plan)?;
        plan.validate().map_err(issue_error)?;
        let (repository, identity, _) = self.issue_repository_identity()?;
        if identity != plan.repository {
            return Err(issue_error(
                "Issue repository changed after planning".into(),
            ));
        }
        let existing = self
            .issue_assignment_store()?
            .existing_batch(params.command_id.as_str())
            .map_err(issue_error)?;
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| issue_error("Issue configuration unavailable".into()))?
            .read_snapshot()
            .map_err(|error| issue_error(error.to_string()))?;
        if existing.is_none()
            && (config.revision.get() != plan.config_revision
                || config
                    .values
                    .issues
                    .repositories
                    .get(&identity.key())
                    .cloned()
                    .unwrap_or_default()
                    != plan.workflow)
        {
            return Err(issue_error(
                "Issue workflow changed; review a fresh plan".into(),
            ));
        }
        let runtime = self.turn_changes_runtime()?;
        let github = zeta_github::GitHub::default();
        for expected in plan.items.iter().flat_map(|item| &item.issues) {
            if existing.is_some() {
                break;
            }
            let actual = runtime
                .worktree_runtime
                .block_on(github.issue_metadata(&repository, expected.number))
                .map_err(issue_error)?;
            let snapshot = runtime
                .worktree_runtime
                .block_on(github.issue(&repository, expected.number))
                .map_err(issue_error)?;
            if super::issue_assignment::issue_material_digest(&snapshot)?
                != expected.material_digest
                || actual.node_id != expected.node_id
                || actual.issue.state != "open"
                || actual.issue.updated_at != expected.updated_at
            {
                return Err(issue_error(format!(
                    "Issue #{} changed after planning; review it again",
                    expected.number
                )));
            }
            if actual
                .issue
                .assignees
                .iter()
                .any(|assignee| !assignee.login.eq_ignore_ascii_case(&plan.workflow.assignee))
            {
                return Err(issue_error(format!(
                    "Issue #{} has an external owner",
                    expected.number
                )));
            }
        }
        let store = self.issue_assignment_store()?;
        let assignments = if params.action == IssueAssignmentStartAction::CreateBranch {
            store
                .prepare_branches(params.command_id.as_str(), &plan, now()?)
                .map_err(issue_error)?
        } else if params.action == IssueAssignmentStartAction::Execute {
            store
                .start(params.command_id.as_str(), &plan, now()?)
                .map_err(issue_error)?
        } else {
            store
                .claim(params.command_id.as_str(), &plan, now()?)
                .map_err(issue_error)?
        };
        let mut ready = Vec::new();
        for assignment in assignments {
            let id = assignment.id.clone();
            match self.prepare_issue_assignment(&assignment) {
                Ok(prepared) => ready.push(prepared),
                Err(error) => {
                    let latest = store.read(&id).map_err(issue_error)?;
                    let failed = store
                        .apply(
                            &format!("{id}-prepare-failed-{}", latest.revision),
                            &id,
                            latest.revision,
                            IssueAssignmentCommand::SyncFailed {
                                state: IssueSyncState::Pending,
                                detail: error.to_string(),
                            },
                            now()?,
                        )
                        .map_err(issue_error)?;
                    ready.push(failed);
                }
            }
        }
        if params.action == IssueAssignmentStartAction::Execute {
            self.start_issue_batch(&plan, &ready)?;
        }
        self.issue_assignment_views(
            ready
                .into_iter()
                .map(|assignment| store.read(&assignment.id).map_err(issue_error))
                .collect::<Result<_, _>>()?,
        )
    }

    pub(super) fn prepare_issue_assignment(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<IssueAssignment, RpcError> {
        let runtime = self.turn_changes_runtime()?;
        let repository = zeta_github::Repository::new(
            assignment.repository.host.clone(),
            assignment.repository.owner.clone(),
            assignment.repository.name.clone(),
        )
        .map_err(issue_error)?;
        let store = self.issue_assignment_store()?;
        let mut assignment = store.read(&assignment.id).map_err(issue_error)?;
        let github = zeta_github::GitHub::default();
        if assignment.ownership == IssueOwnership::Held && assignment.work_run_id.is_none() {
            assignment = self.sync_issue_assignment(&assignment, IssueStage::Queued)?;
        }
        if assignment.workflow.publication == IssueBranchPublication::Linked
            && assignment.linked_branch_id.is_none()
        {
            let linked = runtime
                .worktree_runtime
                .block_on(github.create_linked_issue_branch(
                    &repository,
                    &assignment.item.issues[0].node_id,
                    &assignment.branch,
                    &assignment.base_commit,
                ))
                .map_err(issue_error)?;
            assignment = store
                .apply(
                    &format!("{}-branch", assignment.id),
                    &assignment.id,
                    assignment.revision,
                    IssueAssignmentCommand::RecordBranch {
                        linked_branch_id: linked.id,
                    },
                    now()?,
                )
                .map_err(issue_error)?;
        }
        self.prepare_issue_assignment_thread(&assignment)?;
        store.read(&assignment.id).map_err(issue_error)
    }

    pub(super) fn sync_issue_assignment(
        &self,
        assignment: &IssueAssignment,
        stage: IssueStage,
    ) -> Result<IssueAssignment, RpcError> {
        sync_assignment(
            self.issue_assignment_store()?,
            self.turn_changes_runtime()?.as_ref(),
            assignment,
            stage,
            &zeta_async_utils::CancellationSource::new().token(),
        )
    }

    pub(super) fn issue_assignment_action(&self, value: &Value) -> Result<Value, RpcError> {
        let params: IssueAssignmentActionParams = decode(value)?;
        let store = self.issue_assignment_store()?;
        if let Some(receipt) = store
            .action_receipt(params.command_id.as_str(), value)
            .map_err(issue_error)?
        {
            return Ok(receipt);
        }
        let requested_control = match &params.action {
            IssueAssignmentAction::Pause => Some(zeta_state::IssueControl::Pause),
            IssueAssignmentAction::Release => Some(zeta_state::IssueControl::Release),
            IssueAssignmentAction::Cancel => Some(zeta_state::IssueControl::Cancel),
            IssueAssignmentAction::Transfer { assignee } => {
                Some(zeta_state::IssueControl::Transfer(assignee.clone()))
            }
            _ => None,
        };
        if let Some(control) = requested_control {
            if let Some(receipt) = store
                .command_receipt(params.command_id.as_str())
                .map_err(issue_error)?
            {
                let applied = store
                    .apply(
                        params.command_id.as_str(),
                        &params.assignment_id,
                        receipt.revision.saturating_sub(1),
                        IssueAssignmentCommand::Control {
                            epoch: params.expected_epoch,
                            action: control,
                        },
                        now()?,
                    )
                    .map_err(issue_error)?;
                let mut prior = applied.clone();
                prior.epoch = params.expected_epoch;
                self.interrupt_issue_assignment(&prior)?;
                self.ensure_issue_scheduler(&applied)?;
                let response = self
                    .issue_assignment_views(vec![store.read(&applied.id).map_err(issue_error)?])?;
                store
                    .record_action(params.command_id.as_str(), value, &response)
                    .map_err(issue_error)?;
                return Ok(response);
            }
        }
        let mut assignment = store.read(&params.assignment_id).map_err(issue_error)?;
        let local_control = matches!(
            params.action,
            IssueAssignmentAction::Pause
                | IssueAssignmentAction::Cancel
                | IssueAssignmentAction::Release
                | IssueAssignmentAction::RetrySync
        );
        if !local_control && assignment.repository != self.issue_repository_identity()?.1 {
            return Err(issue_error("Issue repository changed".into()));
        }
        if assignment.epoch != params.expected_epoch
            || !local_control
                && params.action != IssueAssignmentAction::Verify
                && assignment.revision != params.expected_revision
        {
            return Err(issue_error(
                "Issue assignment changed; refresh before acting".into(),
            ));
        }
        match params.action {
            IssueAssignmentAction::RetrySync => {
                assignment = store
                    .apply(
                        params.command_id.as_str(),
                        &assignment.id,
                        assignment.revision,
                        IssueAssignmentCommand::PrepareSync {
                            stage: assignment.desired_stage,
                        },
                        now()?,
                    )
                    .map_err(issue_error)?;
                if assignment.work_run_id.is_none() {
                    assignment = self.prepare_issue_assignment(&assignment)?;
                    if assignment.auto_start {
                        self.resume_issue_assignment(&assignment)?;
                    }
                }
                self.ensure_issue_scheduler(&assignment)?;
            }
            IssueAssignmentAction::Resume => {
                self.resume_issue_assignment(&assignment)?;
                assignment = store.read(&assignment.id).map_err(issue_error)?;
            }
            IssueAssignmentAction::Verify => {
                self.verify_issue_assignment(&assignment)?;
            }
            IssueAssignmentAction::Deliver => {
                self.deliver_issue_assignment(&assignment)?;
            }
            action => {
                if let IssueAssignmentAction::Transfer { assignee } = &action {
                    let (repository, _, _) = self.issue_repository_identity()?;
                    let runtime = self.turn_changes_runtime()?;
                    let allowed = runtime
                        .worktree_runtime
                        .block_on(zeta_github::GitHub::default().issue_assignees(&repository))
                        .map_err(issue_error)?;
                    if !allowed
                        .iter()
                        .any(|account| account.login.eq_ignore_ascii_case(assignee))
                    {
                        return Err(issue_error("Choose an assignable GitHub account".into()));
                    }
                }
                let control = match action {
                    IssueAssignmentAction::Pause => zeta_state::IssueControl::Pause,
                    IssueAssignmentAction::Release => zeta_state::IssueControl::Release,
                    IssueAssignmentAction::Cancel => zeta_state::IssueControl::Cancel,
                    IssueAssignmentAction::Transfer { assignee } => {
                        zeta_state::IssueControl::Transfer(assignee)
                    }
                    _ => unreachable!(),
                };
                let prior = assignment.clone();
                assignment = store
                    .apply(
                        params.command_id.as_str(),
                        &assignment.id,
                        assignment.revision,
                        IssueAssignmentCommand::Control {
                            epoch: assignment.epoch,
                            action: control,
                        },
                        now()?,
                    )
                    .map_err(issue_error)?;
                self.interrupt_issue_assignment(&prior)?;
            }
        }
        self.ensure_issue_scheduler(&assignment)?;
        let response =
            self.issue_assignment_views(vec![store.read(&assignment.id).map_err(issue_error)?])?;
        store
            .record_action(params.command_id.as_str(), value, &response)
            .map_err(issue_error)?;
        Ok(response)
    }

    pub(super) fn issue_assignment_views(
        &self,
        assignments: Vec<IssueAssignment>,
    ) -> Result<Value, RpcError> {
        let now = now()?;
        let assignments = assignments
            .into_iter()
            .map(|assignment| {
                let mut stage = match assignment.ownership {
                    IssueOwnership::Unclaimed | IssueOwnership::Released => IssueStage::Todo,
                    IssueOwnership::Completed => IssueStage::Completed,
                    IssueOwnership::Cancelled => IssueStage::Cancelled,
                    IssueOwnership::Held
                    | IssueOwnership::Releasing
                    | IssueOwnership::Transferring => IssueStage::Queued,
                };
                let mut health = if assignment
                    .lease_until
                    .is_some_and(|deadline| now >= deadline)
                {
                    "Disconnected"
                } else {
                    "Idle"
                }
                .to_string();
                if let (Some(runtime), Some(run_id), Some(attempt_id)) = (
                    &self.work_coordination,
                    &assignment.work_run_id,
                    &assignment.attempt_id,
                ) {
                    let run = runtime
                        .read(run_id)
                        .map_err(|error| issue_error(error.to_string()))?;
                    if let Some(attempt) = run.attempts.get(attempt_id) {
                        use zeta_work_coordination::WorkAttemptExecutionStatus as Status;
                        stage = match attempt.execution_status {
                            Status::Planned | Status::Waiting => IssueStage::Queued,
                            Status::Exploring | Status::Writing => IssueStage::InProgress,
                            Status::Sealed => IssueStage::Review,
                            Status::Failed | Status::Interrupted => IssueStage::Blocked,
                            Status::Cancelled => IssueStage::Cancelled,
                        };
                        if let Some(thread) = &assignment.thread_id {
                            let snapshot = self
                                .threads
                                .read_thread(thread)
                                .map_err(super::core_error)?;
                            if let Some(turn) = snapshot.turns.last() {
                                health = format!("{:?}", turn.status);
                            }
                        }
                    }
                }
                if let Some(receipt) = &assignment.delivery {
                    health = if receipt.pull_request_number.is_some() {
                        "PR awaiting merge".into()
                    } else {
                        "Verified · ready to deliver".into()
                    };
                }
                if matches!(
                    assignment.ownership,
                    IssueOwnership::Unclaimed
                        | IssueOwnership::Released
                        | IssueOwnership::Completed
                        | IssueOwnership::Cancelled
                ) {
                    stage = match assignment.ownership {
                        IssueOwnership::Completed => IssueStage::Completed,
                        IssueOwnership::Cancelled => IssueStage::Cancelled,
                        _ => IssueStage::Todo,
                    };
                }
                if assignment.ownership == IssueOwnership::Held
                    && assignment
                        .lease_until
                        .is_some_and(|deadline| now >= deadline)
                {
                    health = "Disconnected".into();
                }
                if assignment.ownership == IssueOwnership::Releasing {
                    health = "Releasing ownership".into();
                }
                if assignment.ownership == IssueOwnership::Transferring {
                    health = format!(
                        "Transferring to @{}",
                        assignment.pending_owner.as_deref().unwrap_or("")
                    );
                }
                if assignment.ownership == IssueOwnership::Completed {
                    health = "Delivered".into();
                }
                if assignment.ownership == IssueOwnership::Released {
                    health = "Released".into();
                }
                if assignment.ownership == IssueOwnership::Cancelled {
                    health = "Cancelled".into();
                }
                if assignment.paused && assignment.ownership == IssueOwnership::Held {
                    health = "Paused".into();
                }
                if assignment.execution_error.is_some() {
                    health = "Blocked".into();
                }
                if assignment.sync_state == IssueSyncState::Conflict {
                    health = "GitHub conflict".into();
                }
                Ok(IssueAssignmentView {
                    branch_url: assignment.linked_branch_id.as_ref().map(|_| {
                        format!(
                            "https://{}/{}/{}/tree/{}",
                            assignment.repository.host,
                            assignment.repository.owner,
                            assignment.repository.name,
                            assignment.branch
                        )
                    }),
                    assignment: convert(&assignment)?,
                    stage: convert(&stage)?,
                    health,
                    pull_request_url: assignment
                        .delivery
                        .as_ref()
                        .and_then(|receipt| receipt.pull_request_url.clone()),
                })
            })
            .collect::<Result<_, RpcError>>()?;
        result(&IssueAssignmentsResult { assignments })
    }
}

pub(super) fn sync_assignment(
    store: &zeta_state::SqliteIssueAssignmentStore,
    runtime: &super::turn_changes_runtime::TurnChangesRuntime,
    assignment: &IssueAssignment,
    stage: IssueStage,
    cancellation: &zeta_async_utils::CancellationToken,
) -> Result<IssueAssignment, RpcError> {
    let latest = store.read(&assignment.id).map_err(issue_error)?;
    if latest.epoch != assignment.epoch {
        return Err(issue_error("Issue synchronization was superseded".into()));
    }
    let recovering = !latest.attempted_stages.is_empty();
    let pending = store
        .apply(
            &format!("{}-sync-begin-{}", latest.id, latest.revision),
            &latest.id,
            latest.revision,
            IssueAssignmentCommand::BeginSync {
                epoch: latest.epoch,
                stage,
            },
            now()?,
        )
        .map_err(issue_error)?;
    let assignment = &pending;
    let repository = zeta_github::Repository::new(
        assignment.repository.host.clone(),
        assignment.repository.owner.clone(),
        assignment.repository.name.clone(),
    )
    .map_err(issue_error)?;
    let github = zeta_github::GitHub::default();
    let managed = assignment
        .workflow
        .labels
        .all()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let next_owner = if assignment.ownership == IssueOwnership::Transferring {
        let owner = assignment
            .pending_owner
            .as_ref()
            .ok_or_else(|| issue_error("Transfer omitted its new owner".into()))?;
        let allowed = runtime
            .worktree_runtime
            .block_on(sync_call(github.issue_assignees(&repository), cancellation))
            .map_err(issue_error)?;
        if !allowed
            .iter()
            .any(|account| account.login.eq_ignore_ascii_case(owner))
        {
            return Err(issue_error(
                "Transfer target cannot be assigned in this repository".into(),
            ));
        }
        Some(owner)
    } else {
        None
    };
    let mut synced = BTreeMap::new();
    for issue in &assignment.item.issues {
        if store.read(&assignment.id).map_err(issue_error)?.epoch != assignment.epoch {
            return Err(issue_error("Issue synchronization was superseded".into()));
        }
        let before = runtime
            .worktree_runtime
            .block_on(sync_call(
                github.issue_metadata(&repository, issue.number),
                cancellation,
            ))
            .map_err(issue_error)?;
        if before.node_id != issue.node_id
            || before.issue.state != "open"
                && !matches!(
                    assignment.ownership,
                    IssueOwnership::Releasing | IssueOwnership::Cancelled
                )
        {
            let reason = format!("Issue #{} closed or changed identity", issue.number);
            let current = store.read(&assignment.id).map_err(issue_error)?;
            if current.epoch == assignment.epoch {
                store
                    .apply(
                        &format!("{}-identity-conflict-{}", current.id, current.revision),
                        &current.id,
                        current.revision,
                        IssueAssignmentCommand::SyncFailed {
                            state: IssueSyncState::Conflict,
                            detail: reason.clone(),
                        },
                        now()?,
                    )
                    .map_err(issue_error)?;
            }
            return Err(issue_error(reason));
        }
        if assignment.ownership == IssueOwnership::Held {
            let snapshot = runtime
                .worktree_runtime
                .block_on(sync_call(
                    github.issue(&repository, issue.number),
                    cancellation,
                ))
                .map_err(issue_error)?;
            let expected_owner = assignment.synced_labels.contains_key(&issue.node_id);
            let changed_owner = expected_owner
                && (before.issue.assignees.len() != 1
                    || !before.issue.assignees[0]
                        .login
                        .eq_ignore_ascii_case(&assignment.owner));
            let actual_labels = before
                .issue
                .labels
                .iter()
                .filter(|label| managed.contains(&label.name))
                .map(|label| label.name.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let expected_labels = assignment
                .synced_labels
                .get(&issue.node_id)
                .into_iter()
                .flatten()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            let next_labels = assignment
                .workflow
                .labels
                .for_stage(stage)
                .map(str::to_owned)
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            let changed_labels = expected_owner
                && actual_labels != expected_labels
                && actual_labels != next_labels
                && !(recovering && actual_labels.is_empty());
            if changed_owner
                || changed_labels
                || super::issue_assignment::issue_material_digest(&snapshot)?
                    != issue.material_digest
            {
                let current = store.read(&assignment.id).map_err(issue_error)?;
                let reason = format!(
                    "Issue #{} requirements, owner or managed stage changed externally; review before continuing",
                    issue.number
                );
                store
                    .apply(
                        &format!("{}-external-{}", current.id, current.revision),
                        &current.id,
                        current.revision,
                        IssueAssignmentCommand::SyncFailed {
                            state: IssueSyncState::Conflict,
                            detail: reason.clone(),
                        },
                        now()?,
                    )
                    .map_err(issue_error)?;
                return Err(issue_error(reason));
            }
        }
        let mut preserve_labels = false;
        if assignment.ownership == IssueOwnership::Releasing {
            preserve_labels = before.issue.assignees.iter().any(|account| {
                !account.login.eq_ignore_ascii_case(&assignment.owner)
                    && !assignment
                        .pending_owner
                        .as_ref()
                        .is_some_and(|owner| account.login.eq_ignore_ascii_case(owner))
            });
            runtime
                .worktree_runtime
                .block_on(sync_call(
                    github.unassign_issue(&repository, issue.number, &assignment.owner),
                    cancellation,
                ))
                .map_err(issue_error)?;
            if let Some(owner) = &assignment.pending_owner {
                runtime
                    .worktree_runtime
                    .block_on(sync_call(
                        github.unassign_issue(&repository, issue.number, owner),
                        cancellation,
                    ))
                    .map_err(issue_error)?;
            }
        } else if let Some(owner) = next_owner {
            if before.issue.assignees.iter().any(|account| {
                !account.login.eq_ignore_ascii_case(&assignment.owner)
                    && !account.login.eq_ignore_ascii_case(owner)
            }) {
                return Err(issue_error(
                    "Issue was assigned outside this transfer".into(),
                ));
            }
            runtime
                .worktree_runtime
                .block_on(sync_call(
                    github.unassign_issue(&repository, issue.number, &assignment.owner),
                    cancellation,
                ))
                .map_err(issue_error)?;
            runtime
                .worktree_runtime
                .block_on(sync_call(
                    github.assign_issue(&repository, issue.number, owner),
                    cancellation,
                ))
                .map_err(issue_error)?;
        } else if before.issue.state == "open" {
            runtime
                .worktree_runtime
                .block_on(sync_call(
                    github.assign_issue(&repository, issue.number, &assignment.owner),
                    cancellation,
                ))
                .map_err(issue_error)?;
        }
        if preserve_labels {
            synced.insert(
                issue.node_id.clone(),
                before
                    .issue
                    .labels
                    .iter()
                    .filter(|label| managed.contains(&label.name))
                    .map(|label| label.name.clone())
                    .collect(),
            );
            continue;
        }
        let mut expected = match assignment.synced_labels.get(&issue.node_id) {
            Some(labels) => labels.clone(),
            None => {
                let initial = before
                    .issue
                    .labels
                    .iter()
                    .filter(|label| managed.contains(&label.name))
                    .map(|label| label.name.clone())
                    .collect::<Vec<_>>();
                if initial.iter().any(|name| {
                    name != &assignment.workflow.labels.todo
                        && name != &assignment.workflow.labels.queued
                }) {
                    return Err(issue_error(
                        "Issue already carries an active external stage".into(),
                    ));
                }
                initial
            }
        };
        let actual = before
            .issue
            .labels
            .iter()
            .filter(|label| managed.contains(&label.name))
            .map(|label| label.name.clone())
            .collect::<Vec<_>>();
        if actual.len() <= 1
            && assignment.attempted_stages.iter().any(|attempted| {
                assignment
                    .workflow
                    .labels
                    .for_stage(*attempted)
                    .map(|name| vec![name.to_owned()])
                    .unwrap_or_default()
                    == actual
            })
        {
            expected = actual;
        }
        if store.read(&assignment.id).map_err(issue_error)?.epoch != assignment.epoch {
            return Err(issue_error("Issue synchronization was superseded".into()));
        }
        let labels = runtime
            .worktree_runtime
            .block_on(sync_call(
                github.sync_issue_labels(
                    &repository,
                    issue.number,
                    &managed,
                    if before.issue.state == "open" {
                        assignment.workflow.labels.for_stage(stage)
                    } else {
                        None
                    },
                    &expected,
                ),
                cancellation,
            ))
            .map_err(issue_error)?;
        synced.insert(issue.node_id.clone(), labels);
    }
    let latest = store.read(&assignment.id).map_err(issue_error)?;
    if latest.epoch != assignment.epoch
        || latest.owner != assignment.owner
        || latest.ownership != assignment.ownership
    {
        return Err(issue_error(
            "Issue ownership changed during synchronization".into(),
        ));
    }
    let synced = store
        .apply(
            &format!("{}-sync-{}", assignment.id, latest.revision),
            &assignment.id,
            latest.revision,
            IssueAssignmentCommand::RecordSync {
                stage,
                labels: synced,
            },
            now()?,
        )
        .map_err(issue_error)?;
    match synced.ownership {
        IssueOwnership::Releasing => store
            .apply(
                &format!("{}-release-{}", synced.id, synced.epoch),
                &synced.id,
                synced.revision,
                IssueAssignmentCommand::Release,
                now()?,
            )
            .map_err(issue_error),
        IssueOwnership::Transferring => store
            .apply(
                &format!("{}-transfer-{}", synced.id, synced.epoch),
                &synced.id,
                synced.revision,
                IssueAssignmentCommand::Transfer {
                    owner: synced
                        .pending_owner
                        .clone()
                        .ok_or_else(|| issue_error("Transfer owner missing".into()))?,
                },
                now()?,
            )
            .map_err(issue_error),
        _ => Ok(synced),
    }
}

pub(super) async fn sync_call<T>(
    future: impl std::future::Future<Output = Result<T, String>>,
    cancellation: &zeta_async_utils::CancellationToken,
) -> Result<T, String> {
    tokio::select! { result = future => result, _ = cancellation.cancelled() => Err("Issue synchronization stopped".into()) }
}
