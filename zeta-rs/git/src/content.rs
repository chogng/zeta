use std::ffi::OsString;
use std::path::Path;

use crate::{GitClient, GitError, GitRepository, GitResult};

/// Repository revision from which a host wants to read one file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitFileRevision {
    /// The file recorded by the current `HEAD` commit.
    Head,
    /// The file currently recorded in the index.
    Index,
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
