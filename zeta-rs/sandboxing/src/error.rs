use crate::SandboxKind;
use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SandboxError {
    OutsideWorkspace(PathBuf),
    InvalidRelativePath(PathBuf),
    BackendUnavailable {
        backend: SandboxKind,
        message: String,
    },
    Io(String),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideWorkspace(path) => {
                write!(formatter, "path is outside workspace: {}", path.display())
            }
            Self::InvalidRelativePath(path) => {
                write!(
                    formatter,
                    "path must be a relative path without '..': {}",
                    path.display()
                )
            }
            Self::BackendUnavailable { backend, message } => {
                write!(formatter, "{backend:?} sandbox is unavailable: {message}")
            }
            Self::Io(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for SandboxError {}
