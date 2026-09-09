use super::assignment::Action;
use super::assignment::Label;
use super::assignment::PlanMode;
use super::assignment::Reply;
use super::assignment::Request;
use super::assignment::StartAction;
use super::assignment::View;
use super::assignment::Workflow;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::issue_assignment as rpc;

pub(super) fn execute(
    client: &mut AppServerRequestHandle,
    request: Request,
) -> Result<Reply, String> {
    match request {
        Request::Batch { actions } => {
            let mut failures = Vec::new();
            for action in actions {
                if let Err(error) = execute(client, action) {
                    failures.push(error);
                }
            }
            if !failures.is_empty() {
                return Err(format!(
                    "Some batch actions need attention: {}",
                    failures.join("; ")
                ));
            }
            execute(client, Request::List)
        }
        Request::Workflow => read_workflow(client).map(Reply::Workflow),
        Request::SaveWorkflow {
            command_id,
            workflow,
        } => {
            for (name, color) in &workflow.settings.label_colors {
                let existing = workflow
                    .labels
                    .iter()
                    .find(|label| &label.name == name)
                    .ok_or("Create the configured labels first")?;
                if &existing.color != color {
                    client
                        .issue_label_create(rpc::IssueLabelCreateParams {
                            repository: convert(&workflow.repository)?,
                            name: name.clone(),
                            color: color.clone(),
                            expected_color: Some(existing.color.clone()),
                        })
                        .map_err(|error| error.to_string())?;
                }
            }
            client
                .issue_workflow_configure(rpc::IssueWorkflowConfigureParams {
                    command_id,
                    expected_revision: workflow.revision,
                    repository: convert(&workflow.repository)?,
                    workflow: convert(&workflow.settings)?,
                })
                .map_err(|error| error.to_string())?;
            read_workflow(client).map(Reply::Workflow)
        }
        Request::CreateLabels(workflow) => {
            for (name, color) in workflow
                .settings
                .labels
                .all()
                .into_iter()
                .zip(["cfd3d7", "d4c5f9", "1d76db", "0e8a16", "b60205"])
            {
                if workflow.labels.iter().any(|label| label.name == name) {
                    continue;
                }
                client
                    .issue_label_create(rpc::IssueLabelCreateParams {
                        repository: convert(&workflow.repository)?,
                        name: name.into(),
                        color: workflow
                            .settings
                            .label_colors
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| color.into()),
                        expected_color: None,
                    })
                    .map_err(|error| error.to_string())?;
            }
            let mut updated = read_workflow(client)?;
            updated.settings = workflow.settings;
            updated.revision = workflow.revision;
            Ok(Reply::Workflow(updated))
        }
        Request::Plan { numbers, mode } => client
            .issue_plan(rpc::IssuePlanParams {
                numbers,
                mode: match mode {
                    PlanMode::Branch => rpc::IssuePlanMode::Branch,
                    PlanMode::Combined => rpc::IssuePlanMode::Combined,
                    PlanMode::Distributed => rpc::IssuePlanMode::Distributed,
                },
            })
            .map_err(|error| error.to_string())
            .and_then(|result| convert(&result.plan))
            .map(Reply::Plan),
        Request::Start {
            command_id,
            plan,
            action,
        } => client
            .issue_assignment_start(rpc::IssueAssignmentStartParams {
                command_id,
                plan: convert(&plan)?,
                action: match action {
                    StartAction::CreateBranch => rpc::IssueAssignmentStartAction::CreateBranch,
                    StartAction::Claim => rpc::IssueAssignmentStartAction::Claim,
                    StartAction::Execute => rpc::IssueAssignmentStartAction::Execute,
                },
            })
            .map_err(|error| error.to_string())
            .and_then(views)
            .map(Reply::Assignments),
        Request::List => client
            .issue_assignments_list(EmptyParams {})
            .map_err(|error| error.to_string())
            .and_then(views)
            .map(Reply::Assignments),
        Request::Act {
            command_id,
            id,
            revision,
            epoch,
            action,
        } => {
            client
                .issue_assignment_action(rpc::IssueAssignmentActionParams {
                    command_id,
                    assignment_id: id,
                    expected_revision: revision,
                    expected_epoch: epoch,
                    action: match action {
                        Action::Pause => rpc::IssueAssignmentAction::Pause,
                        Action::Resume => rpc::IssueAssignmentAction::Resume,
                        Action::Release => rpc::IssueAssignmentAction::Release,
                        Action::Cancel => rpc::IssueAssignmentAction::Cancel,
                        Action::RetrySync => rpc::IssueAssignmentAction::RetrySync,
                        Action::Verify => rpc::IssueAssignmentAction::Verify,
                        Action::Deliver => rpc::IssueAssignmentAction::Deliver,
                        Action::Transfer(assignee) => {
                            rpc::IssueAssignmentAction::Transfer { assignee }
                        }
                    },
                })
                .map_err(|error| error.to_string())?;
            client
                .issue_assignments_list(EmptyParams {})
                .map_err(|error| error.to_string())
                .and_then(views)
                .map(Reply::Assignments)
        }
    }
}
fn read_workflow(client: &mut AppServerRequestHandle) -> Result<Workflow, String> {
    let result = client
        .issue_workflow_read(EmptyParams {})
        .map_err(|error| error.to_string())?;
    Ok(Workflow {
        repository: convert(&result.repository)?,
        revision: result.config_revision,
        settings: convert(&result.workflow)?,
        default_branch: result.default_branch,
        labels: result
            .labels
            .into_iter()
            .map(|label| Label {
                name: label.name,
                color: label.color,
                id: label.node_id,
            })
            .collect(),
        assignees: result.assignees,
    })
}
pub(super) fn views(result: rpc::IssueAssignmentsResult) -> Result<Vec<View>, String> {
    result
        .assignments
        .into_iter()
        .map(|view| {
            Ok(View {
                assignment: convert(&view.assignment)?,
                stage: convert(&view.stage)?,
                health: view.health,
                branch_url: view.branch_url,
                pr_url: view.pull_request_url,
            })
        })
        .collect()
}
fn convert<T: serde::Serialize, U: serde::de::DeserializeOwned>(value: &T) -> Result<U, String> {
    serde_json::from_value(serde_json::to_value(value).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}
