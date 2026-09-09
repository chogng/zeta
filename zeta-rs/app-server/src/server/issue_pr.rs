use super::AppServer;
use super::RpcError;
use super::core_error;
use super::decode;
use super::git_turn_changes_runtime::GitTurnChangesRuntime;
use super::issue_operations::issue_error;
use super::issue_operations::repository;
use super::result;
use git_turn_changes::CaptureState;
use git_turn_changes::CommitState;
use git_turn_changes::TurnChangeStore;
use github::GitHub;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use zeta_app_server_protocol::protocol::issues::IssuePrCreateParams;
use zeta_app_server_protocol::protocol::issues::IssuePrMode;
use zeta_app_server_protocol::protocol::issues::IssuePrPreview;
use zeta_app_server_protocol::protocol::issues::IssuePrStatus;
use zeta_app_server_protocol::protocol::issues::IssueTaskReadParams;
use zeta_protocol::CommandId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

impl AppServer {
    pub(super) fn issue_pr_preview(&self, value: &Value) -> Result<Value, RpcError> {
        let params: IssueTaskReadParams = decode(value)?;
        let (runtime, task, thread_id) = self.issue_pr_task(&params.session_id)?;
        let binding = runtime
            .binding(&thread_id)
            .ok_or_else(|| issue_error("Task working directory is unavailable".into()))?;
        let preview = runtime
            .dirs
            .runtime
            .block_on(async {
                verify_repository(&task).await?;
                let git = zeta_git::GitClient::system();
                let source = git
                    .open_repository(&task.source_root)
                    .await
                    .map_err(|error| error.to_string())?;
                let checkout = git
                    .open_repository(binding.dir())
                    .await
                    .map_err(|error| error.to_string())?;
                let tree = git
                    .capture_worktree_tree(&checkout)
                    .await
                    .map_err(|error| error.to_string())?;
                let target_commit = git
                    .fetch_main(&source)
                    .await
                    .map_err(|error| error.to_string())?;
                let target_tree = git
                    .resolve_tree(&source, &target_commit)
                    .await
                    .map_err(|error| error.to_string())?;
                let files = git
                    .diff_trees(&source, &target_tree, &tree)
                    .await
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|change| change.path().to_string_lossy().into_owned())
                    .collect();
                let github = GitHub::default();
                let modes = modes(github.merge_options(&task.repository).await?);
                let pull_request = match &task.pull_request {
                    Some(pr) => Some(github.pull_request(&task.repository, pr.number).await?),
                    None => {
                        github
                            .find_pull_request(&task.repository, &task.branch, &task.target_branch)
                            .await?
                    }
                };
                let pull_request = match pull_request {
                    Some(pr) => Some(status(&github, &task.repository, pr, None).await?),
                    None => None,
                };
                Ok::<_, String>(IssuePrPreview {
                    session_id: params.session_id,
                    title: title(&task),
                    body: body(&task),
                    branch: task.branch,
                    target_branch: task.target_branch,
                    start_commit: task.start_commit,
                    target_commit,
                    expected_tree: tree.as_str().into(),
                    files,
                    modes,
                    pull_request,
                })
            })
            .map_err(issue_error)?;
        result(&preview)
    }

    pub(super) fn issue_pr_create(&self, value: &Value) -> Result<Value, RpcError> {
        let params: IssuePrCreateParams = decode(value)?;
        let (runtime, task, thread_id) = self.issue_pr_task(&params.session_id)?;
        let binding = runtime
            .binding(&thread_id)
            .ok_or_else(|| issue_error("Task working directory is unavailable".into()))?;
        runtime
            .dirs
            .runtime
            .block_on(async {
                verify_repository(&task).await?;
                if !modes(GitHub::default().merge_options(&task.repository).await?)
                    .contains(&params.mode)
                {
                    return Err("Repository does not allow the selected merge mode".into());
                }
                let git = zeta_git::GitClient::system();
                let checkout = git
                    .open_repository(binding.dir())
                    .await
                    .map_err(|error| error.to_string())?;
                let tree = git
                    .capture_worktree_tree(&checkout)
                    .await
                    .map_err(|error| error.to_string())?;
                if tree.as_str() != params.expected_tree {
                    return Err(
                        "Files changed after the PR preview; refresh before publishing".into(),
                    );
                }
                Ok::<_, String>(())
            })
            .map_err(issue_error)?;
        commit_changes(&runtime, &task, &thread_id, &params).map_err(issue_error)?;
        let store = self
            .issue_tasks
            .as_ref()
            .ok_or_else(|| issue_error("Issue storage unavailable".into()))?;
        let response = runtime.dirs.runtime.block_on(async {
            let github = GitHub::default();
            let git = zeta_git::GitClient::system();
            let source = git.open_repository(&task.source_root).await.map_err(|error| error.to_string())?;
            let checkout = git.open_repository(binding.dir()).await.map_err(|error| error.to_string())?;
            let current = git.capture_worktree_tree(&checkout).await.map_err(|error| error.to_string())?;
            let head = git.resolve_commit(&source, &format!("refs/heads/{}", task.branch)).await.map_err(|error| error.to_string())?;
            let tree = git.resolve_tree(&source, &head).await.map_err(|error| error.to_string())?;
            if current.as_str() != params.expected_tree || tree.as_str() != params.expected_tree {
                return Err("Task branch does not contain the reviewed files; refresh and resolve pending changes".into());
            }
            let existing = github.find_pull_request(&task.repository, &task.branch, &task.target_branch).await?;
            let pr = match existing {
                Some(pr) => {
                    if pr.head.sha != head { return Err("Existing PR has a different head; review it before publishing more changes".into()); }
                    pr
                }
                None => {
                    git.push_branch_commit(&source, &task.branch, &head).await.map_err(|error| error.to_string())?;
                    github.create_pull_request(&task.repository, github::CreatePullRequest {
                        title: &title(&task), body: &body(&task), head: &task.branch, base: &task.target_branch,
                        draft: params.mode == IssuePrMode::Draft,
                    }).await?
                }
            };
            store.record_pull_request(&task.session_id, &pr)?;
            if pr.head.sha != head {
                return Err(format!("PR {} exists, but its head changed after the reviewed commit; review the PR before continuing", pr.html_url));
            }
            if pr.draft != (params.mode == IssuePrMode::Draft) {
                return Err(format!("Existing PR {} has a different draft state; change its state explicitly on GitHub", pr.html_url));
            }
            let method = match params.mode {
                IssuePrMode::Merge => Some(github::MergeMethod::Merge),
                IssuePrMode::Squash => Some(github::MergeMethod::Squash),
                IssuePrMode::Rebase => Some(github::MergeMethod::Rebase),
                IssuePrMode::Ordinary | IssuePrMode::Draft => None,
            };
            let error = match method { Some(method) => github.enable_auto_merge(&task.repository, &pr, method).await.err(), None => None };
            let refreshed = match github.pull_request(&task.repository, pr.number).await {
                Ok(refreshed) => refreshed,
                Err(refresh_error) => return Ok(IssuePrStatus {
                    url: pr.html_url, state: pr.state, draft: pr.draft, merged: pr.merged_at.is_some(),
                    automatic_merge: None, refresh_error: Some(refresh_error), checks: "Not refreshed".into(), automatic_merge_error: error,
                }),
            };
            store.record_pull_request(&task.session_id, &refreshed)?;
            status(&github, &task.repository, refreshed, error).await
        }).map_err(issue_error)?;
        result(&response)
    }

    fn issue_pr_task(
        &self,
        session_id: &SessionId,
    ) -> Result<(Arc<GitTurnChangesRuntime>, github::IssueTask, ThreadId), RpcError> {
        self.session_view(session_id)?;
        let runtime = self.git_turn_changes_runtime()?;
        let task = self
            .issue_tasks
            .as_ref()
            .ok_or_else(|| issue_error("Issue storage unavailable".into()))?
            .read_session(session_id.as_str())
            .map_err(issue_error)?
            .ok_or_else(|| issue_error("This Session has no associated issues".into()))?;
        if task.source_root != runtime.dirs.root {
            return Err(issue_error(
                "Issue task belongs to another directory".into(),
            ));
        }
        let thread_id = ThreadId::new(session_id.to_string())
            .map_err(|error| issue_error(error.to_string()))?;
        let thread = self.threads.read_thread(&thread_id).map_err(core_error)?;
        if thread.turns.is_empty()
            || thread.turns.iter().any(|turn| {
                matches!(
                    turn.status,
                    zeta_core::TurnStatus::Created
                        | zeta_core::TurnStatus::Running
                        | zeta_core::TurnStatus::WaitingForApproval
                        | zeta_core::TurnStatus::WaitingForUserInput
                        | zeta_core::TurnStatus::WaitingForCapability
                        | zeta_core::TurnStatus::Cancelling
                )
            })
        {
            return Err(issue_error(
                "Finish the development turn before creating a PR".into(),
            ));
        }
        Ok((runtime, task, thread_id))
    }
}

fn commit_changes(
    runtime: &Arc<GitTurnChangesRuntime>,
    task: &github::IssueTask,
    thread_id: &ThreadId,
    params: &IssuePrCreateParams,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(30);
    for initial in runtime
        .store
        .list_for_thread(thread_id)
        .map_err(|error| error.to_string())?
    {
        loop {
            let mut record = runtime
                .store
                .load(&initial.change_set_id)
                .map_err(|error| error.to_string())?;
            if record.files.is_empty()
                || record.capture_state == CaptureState::Discarded
                || matches!(record.commit_state, CommitState::Committed { .. })
            {
                break;
            }
            if record.capture_state != CaptureState::Sealed {
                return Err("Task has an incomplete change capture".into());
            }
            if Instant::now() >= deadline {
                return Err(
                    "Commit preparation is still running; refresh before retrying PR creation"
                        .into(),
                );
            }
            if matches!(
                record.commit_state,
                CommitState::Queued | CommitState::Committing
            ) {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            if let CommitState::Failed { message } = &record.commit_state {
                return Err(format!("Commit failed: {message}"));
            }
            if matches!(record.commit_state, CommitState::Conflict { .. }) {
                return Err("Resolve the task commit conflict before creating a PR".into());
            }
            let operation = format!(
                "{:x}",
                Sha256::digest(format!(
                    "{}:{}:{}",
                    params.command_id, record.change_set_id, record.revision
                ))
            );
            if record.draft_message.is_none() {
                let command_id = CommandId::new(format!("issue-message-{operation}"))
                    .map_err(|error| error.to_string())?;
                let revision = record.revision;
                runtime.update_draft(record, revision, title(task), &command_id, &operation)?;
                record = runtime
                    .store
                    .load(&initial.change_set_id)
                    .map_err(|error| error.to_string())?;
            }
            let command_id = CommandId::new(format!("issue-commit-{operation}"))
                .map_err(|error| error.to_string())?;
            let revision = record.revision;
            runtime.queue_commit(record, revision, &command_id, &operation)?;
        }
    }
    Ok(())
}

async fn verify_repository(task: &github::IssueTask) -> Result<(), String> {
    if repository(&task.source_root).await? != task.repository {
        return Err("Origin repository changed after task creation".into());
    }
    Ok(())
}

fn modes(options: github::MergeOptions) -> Vec<IssuePrMode> {
    let mut modes = vec![IssuePrMode::Ordinary, IssuePrMode::Draft];
    if options.allow_auto_merge {
        if options.allow_merge_commit {
            modes.push(IssuePrMode::Merge);
        }
        if options.allow_squash_merge {
            modes.push(IssuePrMode::Squash);
        }
        if options.allow_rebase_merge {
            modes.push(IssuePrMode::Rebase);
        }
    }
    modes
}

fn title(task: &github::IssueTask) -> String {
    if task.issues.len() == 1 {
        return task.issues[0].issue.title.clone();
    }
    format!(
        "Implement issues {}",
        task.issues
            .iter()
            .map(|issue| format!("#{}", issue.issue.number))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn body(task: &github::IssueTask) -> String {
    task.issues
        .iter()
        .map(|snapshot| {
            format!(
                "- {}\n\nCloses {}",
                snapshot.issue.title, snapshot.issue.html_url
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

async fn status(
    github: &GitHub,
    repository: &github::Repository,
    pr: github::PullRequest,
    automatic_merge_error: Option<String>,
) -> Result<IssuePrStatus, String> {
    let checks = match github.checks(repository, &pr.head.sha).await {
        Ok(checks) => checks,
        Err(error) => format!("Checks unavailable: {error}"),
    };
    Ok(IssuePrStatus {
        url: pr.html_url,
        state: pr.state,
        draft: pr.draft,
        merged: pr.merged_at.is_some(),
        automatic_merge: Some(pr.auto_merge.is_some()),
        refresh_error: None,
        checks,
        automatic_merge_error,
    })
}
