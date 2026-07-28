use std::fmt;
use std::path::PathBuf;

/// Failure returned by a workspace-scoped filesystem implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileSystemError {
    InvalidPath(PathBuf),
    NotDirectory(PathBuf),
    ReadLimitExceeded { maximum_bytes: usize },
    Io(String),
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(path) => {
                write!(
                    formatter,
                    "path is not available in the workspace: {}",
                    path.display()
                )
            }
            Self::NotDirectory(path) => {
                write!(formatter, "path is not a directory: {}", path.display())
            }
            Self::ReadLimitExceeded { maximum_bytes } => {
                write!(
                    formatter,
                    "file exceeds the {maximum_bytes}-byte read limit"
                )
            }
            Self::Io(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for FileSystemError {}
