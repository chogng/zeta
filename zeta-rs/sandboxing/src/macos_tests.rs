use super::*;

#[test]
fn workspace_write_profile_denies_other_writes_and_network() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let command = SandboxCommand::new("echo", ["hello"], workspace.path());
    let policy = SandboxPolicy::new(FileSystemAccess::WorkspaceWrite, NetworkAccess::Denied);

    let prepared = MacosSeatbeltSandbox::new()
        .prepare(&command, policy, &workspace)
        .unwrap();

    assert_eq!(prepared.kind(), SandboxKind::MacosSeatbelt);
    assert_eq!(prepared.program(), SANDBOX_EXEC);
    let profile = prepared.arguments()[1].to_string_lossy();
    assert!(profile.contains("(deny file-write*)"));
    assert!(profile.contains("(allow file-write* (subpath"));
    assert!(profile.contains("(deny network*)"));
}
