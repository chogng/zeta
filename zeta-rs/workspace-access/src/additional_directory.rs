use crate::AdditionalDirectoryContributionPolicy;
use crate::AdditionalInstructionsPolicy;
use zeta_workspace::WorkspaceRoot;

/// Host source retaining access to one additional directory.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdditionalDirectorySource {
    /// Directory supplied when the host process was launched.
    LaunchArgument,
    /// Directory added for the lifetime of the active product Session.
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
    pub(crate) fn new(root: WorkspaceRoot, source: AdditionalDirectorySource) -> Self {
        Self {
            root,
            sources: vec![source],
        }
    }

    /// Returns the canonical filesystem boundary for this additional directory.
    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    /// Returns the sorted sources whose lifetimes currently retain this directory.
    pub fn sources(&self) -> &[AdditionalDirectorySource] {
        &self.sources
    }

    pub(crate) fn add_source(&mut self, source: AdditionalDirectorySource) -> bool {
        if self.sources.contains(&source) {
            return false;
        }
        self.sources.push(source);
        self.sources.sort_unstable();
        true
    }

    pub(crate) fn remove_source(&mut self, source: AdditionalDirectorySource) -> bool {
        let Some(index) = self
            .sources
            .iter()
            .position(|candidate| *candidate == source)
        else {
            return false;
        };
        self.sources.remove(index);
        true
    }

    pub(crate) fn has_no_sources(&self) -> bool {
        self.sources.is_empty()
    }

    /// Resolves configuration contributions independently from filesystem authorization.
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
