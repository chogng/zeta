use super::*;

#[test]
fn workspace_write_is_a_read_only_root_with_a_writable_workspace_overlay() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let command = SandboxCommand::new("echo", ["hello"], workspace.path());
    let policy = SandboxPolicy::new(FileSystemAccess::WorkspaceWrite, NetworkAccess::Denied);

    let prepared =
        LinuxSandbox::new("/usr/bin/bwrap").prepare_command(&command, policy, &workspace);
    let arguments: Vec<_> = prepared
        .arguments()
        .iter()
        .map(|argument| argument.to_string_lossy())
        .collect();

    assert_eq!(prepared.kind(), SandboxKind::LinuxBubblewrap);
    assert_eq!(prepared.program(), "/usr/bin/bwrap");
    assert!(
        arguments
            .windows(3)
            .any(|args| args == ["--ro-bind", "/", "/"])
    );
    let workspace = workspace.path().to_string_lossy();
    assert!(
        arguments
            .windows(3)
            .any(|args| args == ["--bind", workspace.as_ref(), workspace.as_ref()])
    );
    assert!(arguments.iter().any(|argument| argument == "--unshare-net"));
}
