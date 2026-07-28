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
    for name in PROTECTED_WORKSPACE_METADATA_NAMES {
        let path = workspace.path().join(name);
        assert!(
            profile.contains(&format!(
                "(deny file-write* (literal \"{}\"))",
                path.display()
            )),
            "profile did not protect {name}"
        );
    }
}

#[test]
fn seatbelt_denial_classification_requires_a_platform_marker() {
    let backend = MacosSeatbeltSandbox::new();

    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(1),
            "",
            "touch: Operation not permitted",
        ),
        Some(SandboxProcessDenial::process_may_have_started(
            "macOS Seatbelt denied the sandboxed process operation"
        ))
    );
    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(1),
            "",
            "application returned an error",
        ),
        None
    );
    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(71),
            "",
            "sandbox-exec: sandbox_apply: Operation not permitted",
        ),
        Some(SandboxProcessDenial::before_process_start(
            "macOS Seatbelt could not apply the sandbox profile"
        ))
    );
}
