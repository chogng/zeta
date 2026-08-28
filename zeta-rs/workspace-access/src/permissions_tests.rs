use super::AdditionalDirectoryPermission;
use super::AdditionalDirectoryPermissions;
use zeta_workspace::WorkspaceCapability;

#[test]
fn dependent_permissions_require_file_reading() {
    for permission in [
        AdditionalDirectoryPermission::WriteFiles,
        AdditionalDirectoryPermission::ExecuteCommands,
        AdditionalDirectoryPermission::WatchFileChanges,
        AdditionalDirectoryPermission::UseWorkspaceFiles,
        AdditionalDirectoryPermission::UseWorkspaceSearch,
        AdditionalDirectoryPermission::LoadInstructionsAndAgents,
        AdditionalDirectoryPermission::DiscoverSkills,
        AdditionalDirectoryPermission::DiscoverMcp,
        AdditionalDirectoryPermission::UseLanguageServices,
        AdditionalDirectoryPermission::DiscoverHooks,
        AdditionalDirectoryPermission::DiscoverPlugins,
    ] {
        assert!(AdditionalDirectoryPermissions::new([permission]).is_err());
    }
    assert!(
        AdditionalDirectoryPermissions::new([
            AdditionalDirectoryPermission::ReadFiles,
            AdditionalDirectoryPermission::WriteFiles,
        ])
        .is_ok()
    );
}

#[test]
fn user_permissions_map_to_consumer_capabilities() {
    let permissions = AdditionalDirectoryPermissions::new([
        AdditionalDirectoryPermission::ReadFiles,
        AdditionalDirectoryPermission::LoadInstructionsAndAgents,
        AdditionalDirectoryPermission::DiscoverSkills,
    ])
    .unwrap();

    assert!(permissions.allows_workspace_capability(WorkspaceCapability::InspectRepository));
    assert!(
        permissions.allows_workspace_capability(WorkspaceCapability::LoadExecutableConfiguration)
    );
    assert!(!permissions.allows_workspace_capability(WorkspaceCapability::MutateRepository));
    assert!(permissions.allows_workspace_capability(WorkspaceCapability::DiscoverSkills));
    assert!(!permissions.allows_workspace_capability(WorkspaceCapability::DiscoverHooks));
}
