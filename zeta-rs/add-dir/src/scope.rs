use crate::{AdditionalDirectoryContributionPolicy, AdditionalInstructionsPolicy};
use std::fmt;
use zeta_workspace::WorkspaceRoot;

/// Host source that requested access to one additional directory.
///
/// Sources remain distinct when they select the same canonical root so revoking a session grant
/// cannot remove access that is still supplied by persistent configuration.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdditionalDirectorySource {
    /// Directory supplied when the host process was launched.
    LaunchArgument,
    /// Directory added for the lifetime of the active product session.
    SessionCommand,
    /// File-access-only directory restored from durable user configuration.
    PersistentConfiguration,
}

impl AdditionalDirectorySource {
    fn permits_project_contributions(self) -> bool {
        matches!(self, Self::LaunchArgument | Self::SessionCommand)
    }
}

/// One canonical additional directory and every source currently retaining it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdditionalDirectory {
    root: WorkspaceRoot,
    sources: Vec<AdditionalDirectorySource>,
}

impl AdditionalDirectory {
    /// Returns the canonical filesystem boundary for this additional directory.
    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    /// Returns the sorted sources whose lifetimes currently retain this directory.
    pub fn sources(&self) -> &[AdditionalDirectorySource] {
        &self.sources
    }

    /// Resolves the configuration contribution policy from active sources and host compatibility.
    pub fn contribution_policy(
        &self,
        instructions: AdditionalInstructionsPolicy,
    ) -> AdditionalDirectoryContributionPolicy {
        if !self
            .sources
            .iter()
            .any(|source| source.permits_project_contributions())
        {
            return AdditionalDirectoryContributionPolicy::FileAccessOnly;
        }
        match instructions {
            AdditionalInstructionsPolicy::Exclude => {
                AdditionalDirectoryContributionPolicy::AllowlistedProjectContributions
            }
            AdditionalInstructionsPolicy::Include => {
                AdditionalDirectoryContributionPolicy::AllowlistedProjectContributionsWithInstructions
            }
        }
    }
}

/// Effective directory access scope for one primary working directory.
///
/// The primary root defines project identity. Additional roots extend filesystem access without
/// becoming projects or changing which directory owns complete project configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryAccessScope {
    working_directory: WorkspaceRoot,
    additional_directories: Vec<AdditionalDirectory>,
}

impl DirectoryAccessScope {
    /// Creates a scope containing only its primary working directory.
    pub fn new(working_directory: WorkspaceRoot) -> Self {
        Self {
            working_directory,
            additional_directories: Vec::new(),
        }
    }

    /// Returns the primary working directory that defines project identity.
    pub fn working_directory(&self) -> &WorkspaceRoot {
        &self.working_directory
    }

    /// Returns canonical additional directories in stable path order.
    pub fn additional_directories(&self) -> &[AdditionalDirectory] {
        &self.additional_directories
    }

    /// Returns whether the primary or any additional root contains this exact canonical identity.
    pub fn contains(&self, root: &WorkspaceRoot) -> bool {
        self.working_directory == *root
            || self
                .additional_directories
                .iter()
                .any(|directory| directory.root == *root)
    }

    /// Adds or retains one additional canonical root for the supplied source lifetime.
    pub fn add_directory(
        &mut self,
        root: WorkspaceRoot,
        source: AdditionalDirectorySource,
    ) -> Result<DirectoryScopeMutation, DirectoryScopeError> {
        if root == self.working_directory {
            return Err(DirectoryScopeError::WorkingDirectoryCannotBeAdditional);
        }
        if let Some(directory) = self
            .additional_directories
            .iter_mut()
            .find(|directory| directory.root == root)
        {
            if directory.sources.contains(&source) {
                return Ok(DirectoryScopeMutation::AlreadyPresent);
            }
            directory.sources.push(source);
            directory.sources.sort_unstable();
            return Ok(DirectoryScopeMutation::AddedSource);
        }
        self.additional_directories.push(AdditionalDirectory {
            root,
            sources: vec![source],
        });
        self.additional_directories
            .sort_by(|left, right| left.root.canonical_path().cmp(right.root.canonical_path()));
        Ok(DirectoryScopeMutation::AddedDirectory)
    }

    /// Releases one source without disturbing other lifetimes retaining the same canonical root.
    pub fn remove_directory(
        &mut self,
        root: &WorkspaceRoot,
        source: AdditionalDirectorySource,
    ) -> DirectoryScopeMutation {
        let Some(index) = self
            .additional_directories
            .iter()
            .position(|directory| directory.root == *root)
        else {
            return DirectoryScopeMutation::NotPresent;
        };
        let directory = &mut self.additional_directories[index];
        let Some(source_index) = directory
            .sources
            .iter()
            .position(|candidate| *candidate == source)
        else {
            return DirectoryScopeMutation::NotPresent;
        };
        directory.sources.remove(source_index);
        if directory.sources.is_empty() {
            self.additional_directories.remove(index);
            DirectoryScopeMutation::RemovedDirectory
        } else {
            DirectoryScopeMutation::RemovedSource
        }
    }
}

/// Observable result of an idempotent directory-scope mutation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DirectoryScopeMutation {
    /// A new canonical additional root entered the scope.
    AddedDirectory,
    /// An existing canonical root gained another retaining source.
    AddedSource,
    /// The exact root and source were already active.
    AlreadyPresent,
    /// One source was released while another still retains the root.
    RemovedSource,
    /// The final source was released and the canonical root left the scope.
    RemovedDirectory,
    /// The exact root or source was not active.
    NotPresent,
}

/// Invalid attempt to construct an additional-directory scope.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DirectoryScopeError {
    /// The primary working directory was submitted as an additional directory.
    WorkingDirectoryCannotBeAdditional,
}

impl fmt::Display for DirectoryScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkingDirectoryCannotBeAdditional => {
                formatter.write_str("the working directory cannot also be an additional directory")
            }
        }
    }
}

impl std::error::Error for DirectoryScopeError {}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod tests;
