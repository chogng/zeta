use crate::WorkspaceTrustId;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Failure to establish or project one workspace filesystem boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WorkspacePathError {
    #[error("workspace root does not exist or cannot be resolved: {path}: {message}")]
    RootUnavailable { path: PathBuf, message: String },
    #[error("workspace root is not a directory: {}", .0.display())]
    RootNotDirectory(PathBuf),
    #[error("path must be relative and contain no parent, root, or platform prefix: {}", .0.display())]
    InvalidRelativePath(PathBuf),
    #[error("path is outside the workspace: {}", .0.display())]
    OutsideWorkspace(PathBuf),
}

/// Stable identity and filesystem boundary for one existing workspace directory.
///
/// Identity uses the canonical path. The originally requested absolute path is retained because
/// operating-system watcher events can use a lexical alias such as macOS `/var` while filesystem
/// and Git APIs report `/private/var`.
#[derive(Clone, Debug)]
pub struct WorkspaceRoot {
    requested: PathBuf,
    canonical: PathBuf,
}

impl WorkspaceRoot {
    /// Opens one existing directory and freezes both its requested and canonical namespaces.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WorkspacePathError> {
        let path = path.as_ref();
        let requested =
            std::path::absolute(path).map_err(|error| WorkspacePathError::RootUnavailable {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        let canonical = dunce::canonicalize(&requested).map_err(|error| {
            WorkspacePathError::RootUnavailable {
                path: requested.clone(),
                message: error.to_string(),
            }
        })?;
        let metadata =
            canonical
                .metadata()
                .map_err(|error| WorkspacePathError::RootUnavailable {
                    path: canonical.clone(),
                    message: error.to_string(),
                })?;
        if !metadata.is_dir() {
            return Err(WorkspacePathError::RootNotDirectory(canonical));
        }
        Ok(Self {
            requested,
            canonical,
        })
    }

    /// Returns the absolute path supplied by the host before symlink and platform-alias collapse.
    pub fn requested_path(&self) -> &Path {
        &self.requested
    }

    /// Returns the canonical directory used for identity, containment, and filesystem access.
    pub fn canonical_path(&self) -> &Path {
        &self.canonical
    }

    /// Returns the opaque persistence key for trust decisions about this canonical root.
    pub fn trust_id(&self) -> WorkspaceTrustId {
        WorkspaceTrustId::from_canonical_path(&self.canonical)
    }

    /// Resolves an existing relative path after following symlinks and proving containment.
    pub fn resolve_existing(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<PathBuf, WorkspacePathError> {
        let relative_path = relative_path.as_ref();
        let candidate = self.candidate(relative_path)?;
        let canonical = dunce::canonicalize(&candidate).map_err(|error| {
            WorkspacePathError::RootUnavailable {
                path: candidate,
                message: error.to_string(),
            }
        })?;
        self.ensure_contained(canonical)
    }

    /// Resolves a relative write target while checking its nearest existing ancestor.
    ///
    /// This permits creating a new file but rejects lexical escapes and existing parent symlinks
    /// that leave the workspace.
    pub fn resolve_for_write(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<PathBuf, WorkspacePathError> {
        let relative_path = relative_path.as_ref();
        let candidate = self.candidate(relative_path)?;
        if candidate.exists() {
            return self.resolve_existing(relative_path);
        }

        let mut existing_parent = candidate.as_path();
        while !existing_parent.exists() {
            existing_parent = existing_parent.parent().ok_or_else(|| {
                WorkspacePathError::InvalidRelativePath(relative_path.to_path_buf())
            })?;
        }
        let canonical_parent = dunce::canonicalize(existing_parent).map_err(|error| {
            WorkspacePathError::RootUnavailable {
                path: existing_parent.to_path_buf(),
                message: error.to_string(),
            }
        })?;
        self.ensure_contained(canonical_parent)?;
        Ok(candidate)
    }

    /// Projects an observed absolute path into this workspace's relative namespace.
    ///
    /// Both the requested and canonical root aliases are accepted. The method intentionally does
    /// not canonicalize the observed path because removal events commonly refer to paths that no
    /// longer exist.
    pub fn project_observed_path(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref();
        path.strip_prefix(&self.requested)
            .or_else(|_| path.strip_prefix(&self.canonical))
            .ok()
            .map(Path::to_path_buf)
    }

    /// Returns this root relative to an existing canonical ancestor.
    ///
    /// Git uses this projection when a workspace is nested below a repository worktree.
    pub fn relative_to_existing_ancestor(
        &self,
        ancestor: impl AsRef<Path>,
    ) -> Result<PathBuf, WorkspacePathError> {
        let ancestor = ancestor.as_ref();
        let canonical_ancestor =
            dunce::canonicalize(ancestor).map_err(|error| WorkspacePathError::RootUnavailable {
                path: ancestor.to_path_buf(),
                message: error.to_string(),
            })?;
        self.canonical
            .strip_prefix(&canonical_ancestor)
            .map(Path::to_path_buf)
            .map_err(|_| WorkspacePathError::OutsideWorkspace(canonical_ancestor))
    }

    fn candidate(&self, relative_path: &Path) -> Result<PathBuf, WorkspacePathError> {
        if relative_path.as_os_str().is_empty()
            || relative_path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(WorkspacePathError::InvalidRelativePath(
                relative_path.to_path_buf(),
            ));
        }
        Ok(self.canonical.join(relative_path))
    }

    fn ensure_contained(&self, canonical: PathBuf) -> Result<PathBuf, WorkspacePathError> {
        if canonical.starts_with(&self.canonical) {
            Ok(canonical)
        } else {
            Err(WorkspacePathError::OutsideWorkspace(canonical))
        }
    }
}

impl PartialEq for WorkspaceRoot {
    fn eq(&self, other: &Self) -> bool {
        self.canonical == other.canonical
    }
}

impl Eq for WorkspaceRoot {}

impl Hash for WorkspaceRoot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical.hash(state);
    }
}

#[cfg(test)]
#[path = "root_tests.rs"]
mod tests;
