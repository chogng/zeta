use std::collections::BTreeSet;
use zeta_workspace::WorkspaceCapability;

/// User-visible capability granted to one additional directory for one source lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdditionalDirectoryPermission {
    ReadFiles,
    WriteFiles,
    ExecuteCommands,
    WatchFileChanges,
    UseWorkspaceFiles,
    UseWorkspaceSearch,
    LoadInstructionsAndAgents,
    DiscoverSkills,
    DiscoverMcp,
    UseLanguageServices,
    DiscoverHooks,
    DiscoverPlugins,
}

/// Validated capability set attached to one additional-directory authorization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdditionalDirectoryPermissions {
    entries: BTreeSet<AdditionalDirectoryPermission>,
}

impl AdditionalDirectoryPermissions {
    /// Creates the default permissions used by the interactive `/add-dir` command.
    pub fn local_file_tools() -> Self {
        Self::new([
            AdditionalDirectoryPermission::ReadFiles,
            AdditionalDirectoryPermission::WriteFiles,
        ])
        .expect("the built-in local-file permission set is valid")
    }

    /// Validates a complete permission set.
    ///
    /// Every capability that consumes repository content requires `ReadFiles`. Callers must make
    /// dependency changes explicit instead of relying on implicit permission expansion.
    pub fn new(
        permissions: impl IntoIterator<Item = AdditionalDirectoryPermission>,
    ) -> Result<Self, AdditionalDirectoryPermissionsError> {
        let entries = permissions.into_iter().collect::<BTreeSet<_>>();
        let requires_read = entries
            .iter()
            .any(|permission| !matches!(permission, AdditionalDirectoryPermission::ReadFiles));
        if requires_read && !entries.contains(&AdditionalDirectoryPermission::ReadFiles) {
            return Err(AdditionalDirectoryPermissionsError);
        }
        Ok(Self { entries })
    }

    /// Returns permissions in stable order for protocol and UI mapping.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = AdditionalDirectoryPermission> + '_ {
        self.entries.iter().copied()
    }

    /// Reports whether this set grants one user-visible permission.
    pub fn allows(&self, permission: AdditionalDirectoryPermission) -> bool {
        self.entries.contains(&permission)
    }

    /// Reports whether this set grants the capability requested by a consumer.
    pub fn allows_workspace_capability(&self, capability: WorkspaceCapability) -> bool {
        let permission = match capability {
            WorkspaceCapability::InspectRepository => AdditionalDirectoryPermission::ReadFiles,
            WorkspaceCapability::MutateRepository => AdditionalDirectoryPermission::WriteFiles,
            WorkspaceCapability::ExecuteProcess => AdditionalDirectoryPermission::ExecuteCommands,
            WorkspaceCapability::ObserveFileChanges => {
                AdditionalDirectoryPermission::WatchFileChanges
            }
            WorkspaceCapability::BrowseProductFiles => {
                AdditionalDirectoryPermission::UseWorkspaceFiles
            }
            WorkspaceCapability::SearchRepositoryContent => {
                AdditionalDirectoryPermission::UseWorkspaceSearch
            }
            WorkspaceCapability::LoadExecutableConfiguration => {
                AdditionalDirectoryPermission::LoadInstructionsAndAgents
            }
            WorkspaceCapability::DiscoverSkills => AdditionalDirectoryPermission::DiscoverSkills,
            WorkspaceCapability::UseWorkspaceDeclaredTool => {
                AdditionalDirectoryPermission::DiscoverMcp
            }
            WorkspaceCapability::UseLanguageServices => {
                AdditionalDirectoryPermission::UseLanguageServices
            }
            WorkspaceCapability::DiscoverHooks => AdditionalDirectoryPermission::DiscoverHooks,
            WorkspaceCapability::ActivateWorkspaceExtension => {
                AdditionalDirectoryPermission::DiscoverPlugins
            }
        };
        self.allows(permission)
    }
}

/// A permission set granted capabilities without the required file-reading capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdditionalDirectoryPermissionsError;

impl std::fmt::Display for AdditionalDirectoryPermissionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("additional directory capabilities require read-files permission")
    }
}

impl std::error::Error for AdditionalDirectoryPermissionsError {}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod tests;
