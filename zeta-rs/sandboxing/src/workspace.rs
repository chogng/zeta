use crate::SandboxError;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct WorkspaceRoot(PathBuf);

impl WorkspaceRoot {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SandboxError> {
        Ok(Self(
            path.as_ref()
                .canonicalize()
                .map_err(|error| SandboxError::Io(error.to_string()))?,
        ))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Resolves an existing path after following symlinks and proving it remains in the root.
    pub fn resolve_existing(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<PathBuf, SandboxError> {
        let candidate = self.candidate(relative_path.as_ref())?;
        let canonical = candidate
            .canonicalize()
            .map_err(|error| SandboxError::Io(error.to_string()))?;
        self.ensure_contained(canonical)
    }

    /// Resolves a path that may not exist yet while checking its nearest existing parent.
    ///
    /// This supports creating a new workspace file without permitting a lexical escape or an
    /// existing parent symlink that points outside the workspace.
    pub fn resolve_for_write(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<PathBuf, SandboxError> {
        let candidate = self.candidate(relative_path.as_ref())?;
        if candidate.exists() {
            return self.resolve_existing(relative_path);
        }

        let mut existing_parent = candidate.as_path();
        while !existing_parent.exists() {
            existing_parent = existing_parent.parent().ok_or_else(|| {
                SandboxError::InvalidRelativePath(relative_path.as_ref().to_path_buf())
            })?;
        }
        let canonical_parent = existing_parent
            .canonicalize()
            .map_err(|error| SandboxError::Io(error.to_string()))?;
        self.ensure_contained(canonical_parent)?;
        Ok(candidate)
    }

    /// Backwards-compatible alias for resolving an existing workspace path.
    pub fn resolve(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf, SandboxError> {
        self.resolve_existing(relative_path)
    }

    fn candidate(&self, relative_path: &Path) -> Result<PathBuf, SandboxError> {
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(SandboxError::InvalidRelativePath(
                relative_path.to_path_buf(),
            ));
        }
        Ok(self.0.join(relative_path))
    }

    fn ensure_contained(&self, canonical: PathBuf) -> Result<PathBuf, SandboxError> {
        if canonical.starts_with(&self.0) {
            Ok(canonical)
        } else {
            Err(SandboxError::OutsideWorkspace(canonical))
        }
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
