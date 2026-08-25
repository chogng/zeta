#[cfg(any(unix, windows))]
use super::ConnectionOptions;
use super::LifecycleOutput;
use super::LifecycleStatus;
#[cfg(any(unix, windows))]
use super::WorkspaceTrustSource;
#[cfg(any(unix, windows))]
use super::endpoint::endpoint_identity;

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

    let first_identity = endpoint_identity(first.profile_root());
    let same_identity = endpoint_identity(same.profile_root());
    let second_identity = endpoint_identity(second.profile_root());

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
        endpoint_identity(host.profile_root()),
        endpoint_identity(user.profile_root())
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
        endpoint_identity(first.profile_root()),
        endpoint_identity(second.profile_root())
    );
}

#[test]
fn lifecycle_output_uses_one_stable_camel_case_json_contract() {
    let output = LifecycleOutput {
        status: LifecycleStatus::Running,
        pid: Some(42),
        instance_id: Some("instance".into()),
        daemon_version: "1.2.3".into(),
        endpoint_path: "daemon.sock".into(),
        log_path: "daemon.log".into(),
        app_server_name: Some("zeta-app-server".into()),
        schema_hash: Some("sha256:test".into()),
    };

    assert_eq!(
        serde_json::to_value(output).unwrap(),
        serde_json::json!({
            "status": "running",
            "pid": 42,
            "instanceId": "instance",
            "daemonVersion": "1.2.3",
            "endpointPath": "daemon.sock",
            "logPath": "daemon.log",
            "appServerName": "zeta-app-server",
            "schemaHash": "sha256:test",
        })
    );
}
