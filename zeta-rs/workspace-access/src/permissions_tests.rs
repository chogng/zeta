use super::AdditionalDirectoryPermission;
use super::AdditionalDirectoryPermissions;
use zeta_workspace::WorkspaceCapability;

#[test]
fn dependent_permissions_require_file_reading() {
    assert!(
        AdditionalDirectoryPermissions::new([AdditionalDirectoryPermission::WriteFiles]).is_err()
    );
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
        AdditionalDirectoryPermission::LoadProjectConfiguration,
    ])
    .unwrap();

    assert!(permissions.allows_workspace_capability(WorkspaceCapability::InspectRepository));
    assert!(
        permissions.allows_workspace_capability(WorkspaceCapability::LoadExecutableConfiguration)
    );
    assert!(!permissions.allows_workspace_capability(WorkspaceCapability::MutateRepository));
}
