use super::Command;
use super::Event;
use super::Issue;
use super::Page;
use super::PrMode;
use super::PrPlan;
use super::Repository;
use super::StartPoint;
use crate::sessions::ActiveConversation;
use crate::sessions::ConversationCompletion;
use crate::sessions::ResumeOutcome;
use crate::sessions::finish_conversation_request;
use crate::thread::ThreadSubscription;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_protocol::protocol::issues::IssueListParams;
use zeta_app_server_protocol::protocol::issues::IssuePrCreateParams;
use zeta_app_server_protocol::protocol::issues::IssuePrMode;
use zeta_app_server_protocol::protocol::issues::IssuePrStatus;
use zeta_app_server_protocol::protocol::issues::IssueReadParams;
use zeta_app_server_protocol::protocol::issues::IssueRepository;
use zeta_app_server_protocol::protocol::issues::IssueStartPoint;
use zeta_app_server_protocol::protocol::issues::IssueTaskCreateParams;
use zeta_app_server_protocol::protocol::issues::IssueTaskReadParams;

pub(crate) fn execute(
    client: &mut AppServerRequestHandle,
    command: Command,
) -> Result<Event, String> {
    Ok(match command {
        Command::PreviewPr { generation, session_id } => Event::PrPreviewed { generation, result: client.preview_issue_pr(IssueTaskReadParams { session_id }).map(|preview| PrPlan {
            summary: format!("{} -> {} | {} files | {}", preview.branch, preview.target_branch, preview.files.len(), preview.title),
            expected_tree: preview.expected_tree,
            details: format!("{}\n\n{}\n\n{} -> {}\nStarting commit: {}\nTarget commit: {}\n\nFiles:\n{}", preview.title, preview.body, preview.branch, preview.target_branch, preview.start_commit, preview.target_commit, preview.files.join("\n")),
            modes: preview.modes.into_iter().map(|mode| match mode { IssuePrMode::Ordinary => PrMode::Ordinary, IssuePrMode::Draft => PrMode::Draft, IssuePrMode::Merge => PrMode::Merge, IssuePrMode::Squash => PrMode::Squash, IssuePrMode::Rebase => PrMode::Rebase }).collect(),
            existing: preview.pull_request.map(pr_status),
        }).map_err(|error| error.to_string()) },
        Command::CreatePr { generation, session_id, command_id, expected_tree, mode } => Event::PrCreated { generation, result: client.create_issue_pr(IssuePrCreateParams { session_id, command_id, expected_tree, mode: match mode {
            PrMode::Ordinary => IssuePrMode::Ordinary, PrMode::Draft => IssuePrMode::Draft, PrMode::Merge => IssuePrMode::Merge, PrMode::Squash => IssuePrMode::Squash, PrMode::Rebase => IssuePrMode::Rebase,
        } }).map(pr_status).map_err(|error| error.to_string()) },
        Command::List { generation, page, state } => Event::Listed { generation, page, result: client.list_issues(IssueListParams { page, state: match state { super::IssueState::Open => zeta_app_server_protocol::protocol::issues::IssueState::Open, super::IssueState::Closed => zeta_app_server_protocol::protocol::issues::IssueState::Closed } }).map(|result| Page {
            repository: Repository { host: result.repository.host, owner: result.repository.owner, name: result.repository.name },
            issues: result.issues.into_iter().map(|issue| Issue { number: issue.number, title: issue.title }).collect(),
            next: result.next_page,
        }).map_err(|error| error.to_string()) },
        Command::Read { generation, repository, number } => Event::Read { generation, result: client.read_issue(IssueReadParams { repository: repository_dto(repository), number }).map(|result| {
            let mut text = format!("#{} {}\n{}\n\n{}", result.issue.number, result.issue.title, result.issue.url, result.body);
            for comment in result.comments { text.push_str(&format!("\n\n{}\n{}", comment.url, comment.body)); }
            text
        }).map_err(|error| error.to_string()) },
        Command::Start { .. } => return Err("Issue creation requires conversation ownership".into()),
    })
}

pub(crate) fn start(
    mut client: AppServerRequestHandle,
    mut conversation: ActiveConversation,
    subscription: ThreadSubscription,
    command: Command,
) -> Result<(ConversationCompletion, Vec<u64>), String> {
    let Command::Start {
        command_id,
        repository,
        numbers,
        start,
        ..
    } = command
    else {
        return Err("Expected issue task creation".into());
    };
    let result = client
        .create_issue_task(IssueTaskCreateParams {
            command_id,
            repository: repository_dto(repository),
            numbers,
            start: match start {
                StartPoint::CurrentBranch => IssueStartPoint::CurrentBranch,
                StartPoint::Main => IssueStartPoint::Main,
            },
        })
        .map_err(|error| error.to_string())?;
    let task = result.task.ok_or("Issue task creation returned no task")?;
    let change = match conversation
        .resume_session(&mut client, task.session_id.as_str(), None)
        .map_err(|error| error.to_string())?
    {
        ResumeOutcome::Changed(change) => change,
        ResumeOutcome::Listed(_) => return Err("Issue task did not identify a Session".into()),
    };
    let completion = finish_conversation_request(&mut client, conversation, subscription, change)?;
    Ok((
        completion,
        task.issues
            .into_iter()
            .map(|issue| issue.issue.number)
            .collect(),
    ))
}

fn repository_dto(repository: Repository) -> IssueRepository {
    IssueRepository {
        host: repository.host,
        owner: repository.owner,
        name: repository.name,
    }
}

fn pr_status(status: IssuePrStatus) -> String {
    let state = if status.merged {
        "Merged"
    } else if status.draft {
        "Draft"
    } else if status.state == "open" {
        "Open"
    } else {
        "Closed"
    };
    let automatic = match status.automatic_merge {
        Some(true) => "Automatic merge enabled",
        Some(false) => "Automatic merge not enabled",
        None => "Automatic merge status unavailable",
    };
    let mut text = format!("{}\n{state} | {automatic}\n{}", status.url, status.checks);
    if let Some(error) = status.automatic_merge_error {
        text.push_str(&format!("\nAutomatic merge failed: {error}"));
    }
    if let Some(error) = status.refresh_error {
        text.push_str(&format!("\nRefresh failed: {error}"));
    }
    text
}
