//! Workspace-root validation for locally executed tools.

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct WorkspaceRoot(PathBuf);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SandboxError {
    OutsideWorkspace(PathBuf),
    Io(String),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideWorkspace(path) => {
                write!(formatter, "path is outside workspace: {}", path.display())
            }
            Self::Io(message) => formatter.write_str(message),
        }
    }
}
impl std::error::Error for SandboxError {}

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

    pub fn resolve(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf, SandboxError> {
        let candidate = self.0.join(relative_path);
        let canonical = candidate
            .canonicalize()
            .map_err(|error| SandboxError::Io(error.to_string()))?;
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
