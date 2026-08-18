use std::ffi::OsString;
use std::path::Path;

use crate::{GitChangeStatus, GitClient, GitError, GitRepository, GitRepositoryChange, GitResult};

/// Repository revision from which a host wants to read one file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitFileRevision {
    /// The file recorded by the current `HEAD` commit.
    Head,
    /// The file currently recorded in the index.
    Index,
}

/// Selects the canonical two-sided comparison for one current repository change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitChangeFileComparison {
    /// Compares the current `HEAD` commit with the index.
    Staged,
    /// Compares the index with the working tree.
    Unstaged,
}

/// Bounded before/after bytes used to open one current repository change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitChangeFile {
    original: Option<Vec<u8>>,
    modified: Option<Vec<u8>>,
}

impl GitChangeFile {
    pub fn original(&self) -> Option<&[u8]> {
        self.original.as_deref()
    }

    pub fn modified(&self) -> Option<&[u8]> {
        self.modified.as_deref()
    }
}

impl GitClient {
    /// Reads one repository-relative file at a named Git revision.
    ///
    /// Missing paths, including every path in an unborn `HEAD`, return `Ok(None)`. Hosts use this
    /// distinction to build added and deleted file diffs without parsing Git diagnostics.
    pub async fn read_file_at_revision(
        &self,
        repository: &GitRepository,
        path: &Path,
        revision: GitFileRevision,
        maximum_bytes: usize,
    ) -> GitResult<Option<Vec<u8>>> {
        validate_relative_path(path)?;
        if maximum_bytes == 0 {
            return Err(GitError::InvalidConfiguration {
                field: "maximum_bytes",
                requirement: "must be non-zero",
            });
        }
        let mut object = match revision {
            GitFileRevision::Head => OsString::from("HEAD:"),
            GitFileRevision::Index => OsString::from(":"),
        };
        object.push(path);
        let output = self
            .run_query_unchecked(
                repository.worktree_root(),
                [
                    OsString::from("show"),
                    OsString::from("--no-textconv"),
                    object,
                ],
            )
            .await?;
        if !output.status.success() {
            return if missing_object(&output.stderr) {
                Ok(None)
            } else {
                Err(GitError::CommandFailed {
                    command: output.command,
                    exit_code: output.status.code(),
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
                })
            };
        }
        if output.stdout.len() > maximum_bytes {
            return Err(GitError::OutputLimitExceeded {
                command: output.command,
                stream: "stdout",
                limit_bytes: maximum_bytes,
            });
        }
        Ok(Some(output.stdout))
    }

    /// Reads the exact sides represented by a staged or unstaged status resource.
    pub async fn change_file(
        &self,
        repository: &GitRepository,
        change: &GitRepositoryChange,
        comparison: GitChangeFileComparison,
        maximum_bytes: usize,
    ) -> GitResult<GitChangeFile> {
        if maximum_bytes == 0 {
            return Err(GitError::InvalidConfiguration {
                field: "maximum_bytes",
                requirement: "must be non-zero",
            });
        }
        let original_path = comparison_original_path(change, comparison);
        let original = self
            .read_file_at_revision(
                repository,
                original_path,
                match comparison {
                    GitChangeFileComparison::Staged => GitFileRevision::Head,
                    GitChangeFileComparison::Unstaged => GitFileRevision::Index,
                },
                maximum_bytes,
            )
            .await?;
        let modified = match comparison {
            GitChangeFileComparison::Staged => {
                self.read_file_at_revision(
                    repository,
                    change.path(),
                    GitFileRevision::Index,
                    maximum_bytes,
                )
                .await?
            }
            GitChangeFileComparison::Unstaged => {
                read_worktree_file(repository, change.path(), maximum_bytes)?
            }
        };
        Ok(GitChangeFile { original, modified })
    }
}

fn comparison_original_path(
    change: &GitRepositoryChange,
    comparison: GitChangeFileComparison,
) -> &Path {
    let status = match comparison {
        GitChangeFileComparison::Staged => change.index_status(),
        GitChangeFileComparison::Unstaged => change.worktree_status(),
    };
    if matches!(status, GitChangeStatus::Renamed | GitChangeStatus::Copied) {
        change.original_path().unwrap_or_else(|| change.path())
    } else {
        change.path()
    }
}

fn read_worktree_file(
    repository: &GitRepository,
    path: &Path,
    maximum_bytes: usize,
) -> GitResult<Option<Vec<u8>>> {
    validate_relative_path(path)?;
    let absolute_path = repository.worktree_root().join(path);
    let metadata = match absolute_path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(GitError::io("inspect Git working-tree file", error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(GitError::runtime(
            "read Git working-tree file",
            "path is not a regular file",
        ));
    }
    if metadata.len() > maximum_bytes as u64 {
        return Err(GitError::OutputLimitExceeded {
            command: format!("read working-tree file {}", path.to_string_lossy()),
            stream: "file",
            limit_bytes: maximum_bytes,
        });
    }
    std::fs::read(absolute_path)
        .map(Some)
        .map_err(|error| GitError::io("read Git working-tree file", error))
}

fn validate_relative_path(path: &Path) -> GitResult<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(GitError::runtime(
            "validate Git file path",
            "path must be a non-empty repository-relative path",
        ));
    }
    Ok(())
}

fn missing_object(stderr: &[u8]) -> bool {
    let stderr = String::from_utf8_lossy(stderr);
    stderr.contains("does not exist in")
        || stderr.contains("exists on disk, but not in")
        || stderr.contains("invalid object name")
        || stderr.contains("unknown revision")
        || stderr.contains("bad revision")
        || stderr.contains("Path '")
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
