#[cfg(any(unix, windows))]
use super::ConnectionOptions;
#[cfg(any(unix, windows))]
use super::WorkspaceTrustSource;
#[cfg(any(unix, windows))]
use super::platform::endpoint_identity;

#[test]
#[cfg(any(unix, windows))]
fn endpoint_identity_is_shared_across_workspaces() {
    let profile = tempfile::tempdir().unwrap();
    let first_workspace = tempfile::tempdir().unwrap();
    let second_workspace = tempfile::tempdir().unwrap();
    let first = ConnectionOptions::new(
        profile.path(),
        Some(first_workspace.path().to_path_buf()),
        WorkspaceTrustSource::HostConfiguration,
        None,
    );
    let same = first.clone();
    let second = ConnectionOptions::new(
        profile.path(),
        Some(second_workspace.path().to_path_buf()),
        WorkspaceTrustSource::HostConfiguration,
        None,
    );

    let first_identity = endpoint_identity(&first, profile.path(), Some(first_workspace.path()))
        .expect("first identity is valid");
    let same_identity = endpoint_identity(&same, profile.path(), Some(first_workspace.path()))
        .expect("same identity is valid");
    let second_identity = endpoint_identity(&second, profile.path(), Some(second_workspace.path()))
        .expect("second identity is valid");

    assert_eq!(first_identity, same_identity);
    assert_eq!(first_identity, second_identity);
}

#[test]
#[cfg(any(unix, windows))]
fn endpoint_identity_is_shared_across_workspace_trust_policies() {
    let profile = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let host = ConnectionOptions::new(
        profile.path(),
        Some(workspace.path().to_path_buf()),
        WorkspaceTrustSource::HostConfiguration,
        None,
    );
    let user = ConnectionOptions::new(
        profile.path(),
        Some(workspace.path().to_path_buf()),
        WorkspaceTrustSource::UserConfig,
        None,
    );

    assert_eq!(
        endpoint_identity(&host, profile.path(), Some(workspace.path())).unwrap(),
        endpoint_identity(&user, profile.path(), Some(workspace.path())).unwrap()
    );
}

#[test]
#[cfg(any(unix, windows))]
fn endpoint_identity_is_shared_across_product_service_adapters() {
    let profile = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let first_install = tempfile::tempdir().unwrap();
    let first_manifest = first_install.path().join("product-services.json");
    std::fs::write(&first_manifest, r#"{"schemaVersion":1}"#).unwrap();
    let first = ConnectionOptions::new(
        profile.path(),
        Some(workspace.path().to_path_buf()),
        WorkspaceTrustSource::UserConfig,
        Some(first_manifest),
    );
    let second = ConnectionOptions::new(
        profile.path(),
        Some(workspace.path().to_path_buf()),
        WorkspaceTrustSource::UserConfig,
        None,
    );

    assert_eq!(
        endpoint_identity(&first, profile.path(), Some(workspace.path())).unwrap(),
        endpoint_identity(&second, profile.path(), Some(workspace.path())).unwrap()
    );
}
