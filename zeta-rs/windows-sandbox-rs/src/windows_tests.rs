use super::*;

#[test]
fn resolves_workspace_and_network_authority_for_the_native_launcher() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let policy = SandboxPolicy::new(FileSystemAccess::WorkspaceWrite, NetworkAccess::Denied);

    let plan = WindowsSandbox::new().plan(policy, &workspace);

    assert_eq!(plan.workspace(), workspace.path());
    assert_eq!(plan.file_system(), FileSystemAccess::WorkspaceWrite);
    assert_eq!(plan.network(), NetworkAccess::Denied);
}

#[test]
fn restricted_requests_fail_closed_until_the_native_launcher_is_connected() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let command = SandboxCommand::new("cmd.exe", ["/c", "echo"], workspace.path());
    let policy = SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Denied);

    let error = WindowsSandbox::new()
        .prepare(&command, policy, &workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        SandboxError::BackendUnavailable {
            backend: SandboxKind::WindowsRestrictedToken,
            ..
        }
    ));
}
