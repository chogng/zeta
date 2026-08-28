use std::fmt;
use std::path::PathBuf;
use zeta_workspace::WorkspaceCapability;

/// Failure to mutate or freeze one Workspace access authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkspaceAccessError {
    WorkingDirectoryCannotBeAdditional,
    RevisionConflict {
        expected: u64,
        actual: u64,
    },
    CapabilityUnavailable {
        root: PathBuf,
        capability: WorkspaceCapability,
    },
}

impl fmt::Display for WorkspaceAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkingDirectoryCannotBeAdditional => {
                formatter.write_str("the working directory cannot also be an additional directory")
            }
            Self::RevisionConflict { expected, actual } => write!(
                formatter,
                "Workspace access revision conflict: expected {expected}, actual {actual}"
            ),
            Self::CapabilityUnavailable { root, capability } => write!(
                formatter,
                "Workspace access is not authorized to {capability}: {}",
                root.display()
            ),
        }
    }
}

impl std::error::Error for WorkspaceAccessError {}
