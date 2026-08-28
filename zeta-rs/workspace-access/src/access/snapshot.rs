use zeta_workspace::TrustedWorkspace;
use zeta_workspace::WorkspaceRoot;

/// Monotonic identity of one effective Workspace access scope.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkspaceAccessRevision(u64);

impl WorkspaceAccessRevision {
    pub(crate) fn advance(&mut self) {
        self.0 = self
            .0
            .checked_add(1)
            .expect("Workspace access revision space is not exhausted");
    }

    /// Returns the monotonic numeric value used for equality and diagnostics.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// Observable result of an idempotent Workspace-access mutation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WorkspaceAccessMutation {
    AddedDirectory,
    AddedSource,
    AlreadyPresent,
    RemovedSource,
    RemovedDirectory,
    NotPresent,
}

impl WorkspaceAccessMutation {
    pub(crate) fn changes_scope(self) -> bool {
        matches!(
            self,
            Self::AddedDirectory | Self::AddedSource | Self::RemovedSource | Self::RemovedDirectory
        )
    }
}

/// Immutable capability-bound Workspace roots for one model or tool operation.
#[derive(Clone, Debug)]
pub struct WorkspaceAccessSnapshot {
    revision: WorkspaceAccessRevision,
    working_directory: WorkspaceRoot,
    additional_roots: Vec<TrustedWorkspace>,
}

impl WorkspaceAccessSnapshot {
    pub(crate) fn new(
        revision: WorkspaceAccessRevision,
        working_directory: WorkspaceRoot,
        additional_roots: Vec<TrustedWorkspace>,
    ) -> Self {
        Self {
            revision,
            working_directory,
            additional_roots,
        }
    }

    /// Returns the authority revision frozen by this snapshot.
    pub fn revision(&self) -> WorkspaceAccessRevision {
        self.revision
    }

    /// Returns the primary root that still owns cwd and complete project configuration.
    pub fn working_directory(&self) -> &WorkspaceRoot {
        &self.working_directory
    }

    /// Returns sorted additional roots bound to the capability requested by the consumer.
    pub fn additional_roots(&self) -> &[TrustedWorkspace] {
        &self.additional_roots
    }
}
