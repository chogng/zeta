use std::ffi::OsString;
use std::num::NonZeroUsize;

use crate::GitClient;
use crate::GitCommitSummary;
use crate::GitError;
use crate::GitRemote;
use crate::GitRepository;
use crate::GitResult;

/// The ref kinds that can be shown in a repository graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitReferenceKind {
    LocalBranch,
    RemoteBranch,
}

/// One local or remote-tracking branch ref projected for a repository graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitReference {
    name: String,
    object_id: String,
    kind: GitReferenceKind,
    remote_name: Option<String>,
    current: bool,
}

impl GitReference {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    pub fn kind(&self) -> GitReferenceKind {
        self.kind
    }

    pub fn remote_name(&self) -> Option<&str> {
        self.remote_name.as_deref()
    }

    pub fn is_current(&self) -> bool {
        self.current
    }
}

/// A page of graph data containing reachable commits, branch refs, and configured remotes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitGraph {
    commits: Vec<GitCommitSummary>,
    references: Vec<GitReference>,
    remotes: Vec<GitRemote>,
    has_more: bool,
}

impl GitGraph {
    pub fn commits(&self) -> &[GitCommitSummary] {
        &self.commits
    }

    pub fn references(&self) -> &[GitReference] {
        &self.references
    }

    pub fn remotes(&self) -> &[GitRemote] {
        &self.remotes
    }

    pub const fn has_more(&self) -> bool {
        self.has_more
    }
}

impl GitClient {
    /// Reads one page of local graph history and all branch/remote metadata needed to render it.
    pub async fn graph(
        &self,
        repository: &GitRepository,
        limit: NonZeroUsize,
        skip: usize,
    ) -> GitResult<GitGraph> {
        let references = self.references(repository).await?;
        let (commits, has_more) = self
            .recent_commits_from_all_refs(repository, limit, skip)
            .await?;
        let remotes = self.remotes(repository).await?;
        Ok(GitGraph {
            commits,
            references,
            remotes,
            has_more,
        })
    }

    /// Lists local branches and fetched remote-tracking branches without exposing raw ref paths.
    pub async fn references(&self, repository: &GitRepository) -> GitResult<Vec<GitReference>> {
        let output = self
            .run_query(
                repository.worktree_root(),
                [
                    "for-each-ref",
                    "--format=%(refname)%00%(objectname)%00%(symref)%00%(HEAD)",
                    "refs/heads",
                    "refs/remotes",
                ],
            )
            .await?;
        parse_references(&output.stdout, &output.command)
    }

    async fn recent_commits_from_all_refs(
        &self,
        repository: &GitRepository,
        limit: NonZeroUsize,
        skip: usize,
    ) -> GitResult<(Vec<GitCommitSummary>, bool)> {
        let query_limit = limit
            .get()
            .checked_add(1)
            .ok_or(GitError::InvalidConfiguration {
                field: "limit",
                requirement: "must leave room for a has-more probe",
            })?;
        let limit_arg = format!("-n{query_limit}");
        let skip_arg = format!("--skip={skip}");
        let output = self
            .run_query(
                repository.worktree_root(),
                [
                    OsString::from("log"),
                    OsString::from("--all"),
                    OsString::from("--topo-order"),
                    OsString::from(skip_arg),
                    OsString::from("-z"),
                    OsString::from("--format=%H%x00%P%x00%ct%x00%s"),
                    OsString::from(limit_arg),
                ],
            )
            .await?;
        let mut commits = super::info::parse_commits(&output.stdout, &output.command)?;
        let has_more = commits.len() > limit.get();
        if has_more {
            commits.truncate(limit.get());
        }
        Ok((commits, has_more))
    }
}

fn parse_references(bytes: &[u8], command: &str) -> GitResult<Vec<GitReference>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| GitError::invalid_output(command, "ref output was not UTF-8"))?;
    let mut references = Vec::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let fields = line.split('\0').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(GitError::invalid_output(
                command,
                format!("ref record had {} fields instead of 4", fields.len()),
            ));
        }
        if !fields[2].is_empty() {
            continue;
        }
        let full_name = fields[0];
        let object_id = fields[1];
        if object_id.is_empty() {
            return Err(GitError::invalid_output(
                command,
                "ref record omitted object id",
            ));
        }
        let reference = if let Some(name) = full_name.strip_prefix("refs/heads/") {
            if name.is_empty() {
                return Err(GitError::invalid_output(command, "local ref omitted name"));
            }
            GitReference {
                name: name.to_string(),
                object_id: object_id.to_string(),
                kind: GitReferenceKind::LocalBranch,
                remote_name: None,
                current: fields[3] == "*",
            }
        } else if let Some(name) = full_name.strip_prefix("refs/remotes/") {
            let Some((remote_name, branch_name)) = name.split_once('/') else {
                return Err(GitError::invalid_output(
                    command,
                    "remote ref omitted remote or branch name",
                ));
            };
            if remote_name.is_empty() || branch_name.is_empty() {
                return Err(GitError::invalid_output(
                    command,
                    "remote ref omitted remote or branch name",
                ));
            }
            GitReference {
                name: name.to_string(),
                object_id: object_id.to_string(),
                kind: GitReferenceKind::RemoteBranch,
                remote_name: Some(remote_name.to_string()),
                current: false,
            }
        } else {
            continue;
        };
        references.push(reference);
    }
    Ok(references)
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
