use crate::SandboxKind;
use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SandboxError {
    OutsideDir(PathBuf),
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
            Self::OutsideDir(path) => {
                write!(formatter, "path is outside directory: {}", path.display())
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

impl From<zeta_file_access::DirPathError> for SandboxError {
    fn from(error: zeta_file_access::DirPathError) -> Self {
        match error {
            zeta_file_access::DirPathError::OutsideDir(path) => Self::OutsideDir(path),
            zeta_file_access::DirPathError::InvalidRelativePath(path) => {
                Self::InvalidRelativePath(path)
            }
            error => Self::Io(error.to_string()),
        }
    }
}
