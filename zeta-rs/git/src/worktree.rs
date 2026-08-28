use std::path::Path;
use std::path::PathBuf;

use crate::GitClient;
use crate::GitError;
use crate::GitRepository;
use crate::GitResult;
use crate::path::path_from_git_bytes;

/// Whether one Git worktree inventory entry can be opened now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitWorktreeAvailability {
    Ready,
    Locked { reason: Option<String> },
    Prunable { reason: Option<String> },
}

impl GitWorktreeAvailability {
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Ready | Self::Locked { .. })
    }
}

/// One entry returned by `git worktree list --porcelain -z`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitWorktree {
    checkout_root: PathBuf,
    head: String,
    branch: Option<String>,
    availability: GitWorktreeAvailability,
}

impl GitWorktree {
    pub fn checkout_root(&self) -> &Path {
        &self.checkout_root
    }

    pub fn head(&self) -> &str {
        &self.head
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub const fn availability(&self) -> &GitWorktreeAvailability {
        &self.availability
    }
}

impl GitClient {
    /// Lists every primary, linked, locked, and prunable worktree for one repository.
    pub async fn worktrees(&self, repository: &GitRepository) -> GitResult<Vec<GitWorktree>> {
        let output = self
            .run_query(
                repository.worktree_root(),
                ["worktree", "list", "--porcelain", "-z"],
            )
            .await?;
        parse_worktrees(&output.stdout, &output.command)
    }
}

fn parse_worktrees(output: &[u8], command: &str) -> GitResult<Vec<GitWorktree>> {
    if !output.is_empty() && !output.ends_with(b"\0\0") {
        return Err(GitError::invalid_output(
            command,
            "worktree list omitted its final record separator",
        ));
    }
    let mut result = Vec::new();
    let mut checkout_root = None;
    let mut head = None;
    let mut branch = None;
    let mut availability = GitWorktreeAvailability::Ready;
    for field in output.split(|byte| *byte == 0) {
        if field.is_empty() {
            if let Some(checkout_root) = checkout_root.take() {
                result.push(GitWorktree {
                    checkout_root,
                    head: head.take().ok_or_else(|| {
                        GitError::invalid_output(command, "worktree record omitted HEAD")
                    })?,
                    branch: branch.take(),
                    availability,
                });
                availability = GitWorktreeAvailability::Ready;
            }
            continue;
        }
        if let Some(value) = field.strip_prefix(b"worktree ") {
            if checkout_root.is_some() {
                return Err(GitError::invalid_output(
                    command,
                    "worktree record omitted its separator",
                ));
            }
            checkout_root = Some(path_from_git_bytes(value, command)?);
        } else if let Some(value) = field.strip_prefix(b"HEAD ") {
            require_worktree_start(checkout_root.as_deref(), command)?;
            head = Some(text_field(value, command, "worktree HEAD")?);
        } else if let Some(value) = field.strip_prefix(b"branch refs/heads/") {
            require_worktree_start(checkout_root.as_deref(), command)?;
            branch = Some(text_field(value, command, "worktree branch")?);
        } else if field == b"detached" || field == b"bare" {
            require_worktree_start(checkout_root.as_deref(), command)?;
        } else if let Some(value) = field.strip_prefix(b"locked") {
            require_worktree_start(checkout_root.as_deref(), command)?;
            availability = GitWorktreeAvailability::Locked {
                reason: optional_reason(value, command, "worktree lock reason")?,
            };
        } else if let Some(value) = field.strip_prefix(b"prunable") {
            require_worktree_start(checkout_root.as_deref(), command)?;
            availability = GitWorktreeAvailability::Prunable {
                reason: optional_reason(value, command, "worktree prune reason")?,
            };
        } else {
            return Err(GitError::invalid_output(
                command,
                format!("unknown worktree field: {}", String::from_utf8_lossy(field)),
            ));
        }
    }
    if checkout_root.is_some() {
        return Err(GitError::invalid_output(
            command,
            "worktree list omitted its final record separator",
        ));
    }
    Ok(result)
}

fn require_worktree_start(checkout_root: Option<&Path>, command: &str) -> GitResult<()> {
    if checkout_root.is_none() {
        return Err(GitError::invalid_output(
            command,
            "worktree record field appeared before its path",
        ));
    }
    Ok(())
}

fn text_field(value: &[u8], command: &str, label: &str) -> GitResult<String> {
    std::str::from_utf8(value)
        .map(ToOwned::to_owned)
        .map_err(|_| GitError::invalid_output(command, format!("{label} was not UTF-8")))
}

fn optional_reason(value: &[u8], command: &str, label: &str) -> GitResult<Option<String>> {
    value
        .strip_prefix(b" ")
        .filter(|value| !value.is_empty())
        .map(|value| text_field(value, command, label))
        .transpose()
}

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod tests;
