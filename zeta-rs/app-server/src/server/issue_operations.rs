use super::AppServer;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::issues::IssueComment;
use zeta_app_server_protocol::protocol::issues::IssueListParams;
use zeta_app_server_protocol::protocol::issues::IssueListResult;
use zeta_app_server_protocol::protocol::issues::IssueReadParams;
use zeta_app_server_protocol::protocol::issues::IssueReadResult;
use zeta_app_server_protocol::protocol::issues::IssueRepository;
use zeta_app_server_protocol::protocol::issues::IssueSummary;

impl AppServer {
    pub(super) fn issue_list(&self, params: &Value) -> Result<Value, RpcError> {
        let params: IssueListParams = decode(params)?;
        let runtime = self.turn_changes_runtime()?;
        let repository = runtime
            .worktree_runtime
            .block_on(repository(&runtime.dir_root))
            .map_err(issue_error)?;
        let page = runtime
            .worktree_runtime
            .block_on(zeta_github::GitHub::default().issues(
                &repository,
                match params.state {
                    zeta_app_server_protocol::protocol::issues::IssueState::Open => {
                        zeta_github::IssueState::Open
                    }
                    zeta_app_server_protocol::protocol::issues::IssueState::Closed => {
                        zeta_github::IssueState::Closed
                    }
                },
                params.page,
            ))
            .map_err(issue_error)?;
        result(&IssueListResult {
            repository: IssueRepository {
                host: repository.host,
                owner: repository.owner,
                name: repository.name,
            },
            issues: page.issues.into_iter().map(summary).collect(),
            next_page: page.next_page,
        })
    }

    pub(super) fn issue_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: IssueReadParams = decode(params)?;
        let runtime = self.turn_changes_runtime()?;
        let repository = runtime
            .worktree_runtime
            .block_on(repository(&runtime.dir_root))
            .map_err(issue_error)?;
        if (
            repository.host.as_str(),
            repository.owner.as_str(),
            repository.name.as_str(),
        ) != (
            params.repository.host.as_str(),
            params.repository.owner.as_str(),
            params.repository.name.as_str(),
        ) {
            return Err(issue_error(
                "Repository changed; refresh the issue list".into(),
            ));
        }
        let snapshot = runtime
            .worktree_runtime
            .block_on(zeta_github::GitHub::default().issue(&repository, params.number))
            .map_err(issue_error)?;
        result(&IssueReadResult {
            body: snapshot.issue.body.clone().unwrap_or_default(),
            issue: summary(snapshot.issue),
            comments: snapshot
                .comments
                .into_iter()
                .map(|comment| IssueComment {
                    body: comment.body,
                    url: comment.html_url,
                    updated_at: comment.updated_at,
                })
                .collect(),
        })
    }
}

pub(super) async fn repository(root: &std::path::Path) -> Result<zeta_github::Repository, String> {
    let git = zeta_git::GitClient::system();
    let repository = git
        .open_repository(root)
        .await
        .map_err(|error| error.to_string())?;
    let remotes = git
        .remotes(&repository)
        .await
        .map_err(|error| error.to_string())?;
    let remote = remotes
        .iter()
        .find(|remote| remote.name() == "origin")
        .ok_or("Issue management requires an origin remote")?;
    if !remote.has_single_identity() {
        return Err("Origin fetch and push URLs must identify the same repository".into());
    }
    let identity = remote
        .identity()
        .ok_or("Origin has no supported repository identity")?;
    zeta_github::Repository::new(
        identity.host().into(),
        identity.owner().into(),
        identity.repository().into(),
    )
}

pub(super) fn summary(issue: zeta_github::Issue) -> IssueSummary {
    IssueSummary {
        number: issue.number,
        title: issue.title,
        url: issue.html_url,
        updated_at: issue.updated_at,
        state: issue.state,
    }
}

pub(super) fn issue_error(detail: String) -> RpcError {
    log::warn!("Issue operation failed: {detail}");
    let mut error = RpcError::new(-32070, AppServerErrorName::IssueOperationFailed);
    error.detail = Some(detail);
    error
}
