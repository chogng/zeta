use super::*;
use crate::protocol::{
    ACCESS_FLAG, COMMAND_SEPARATOR, CWD_FLAG, READ_ONLY_ACCESS, SETUP_HELPER_FLAG, WORKSPACE_FLAG,
};

fn sandbox() -> WindowsSandbox {
    WindowsSandbox::new("zeta-command-runner.exe", "zeta-windows-sandbox-setup.exe")
}

#[test]
fn resolves_workspace_and_network_authority_for_the_native_launcher() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let policy = SandboxPolicy::new(FileSystemAccess::WorkspaceWrite, NetworkAccess::Denied);

    let plan = sandbox().plan(policy, &workspace);

    assert_eq!(plan.workspace(), workspace.path());
    assert_eq!(plan.file_system(), FileSystemAccess::WorkspaceWrite);
    assert_eq!(plan.network(), NetworkAccess::Denied);
}

#[test]
fn prepares_appcontainer_runner_with_frozen_setup_and_inner_command() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let command = SandboxCommand::new("rg.exe", ["--files"], workspace.path());
    let policy = SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Denied);

    let prepared = sandbox().prepare(&command, policy, &workspace).unwrap();

    assert_eq!(prepared.kind(), SandboxKind::WindowsAppContainer);
    assert_eq!(prepared.program(), "zeta-command-runner.exe");
    let arguments = prepared
        .arguments()
        .iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(arguments[0], SETUP_HELPER_FLAG);
    assert_eq!(arguments[1], "zeta-windows-sandbox-setup.exe");
    assert_eq!(arguments[2], ACCESS_FLAG);
    assert_eq!(arguments[3], READ_ONLY_ACCESS);
    assert!(arguments.contains(&WORKSPACE_FLAG.to_owned()));
    assert!(arguments.contains(&CWD_FLAG.to_owned()));
    assert_eq!(
        &arguments[arguments
            .iter()
            .position(|argument| argument == COMMAND_SEPARATOR)
            .unwrap()..],
        [COMMAND_SEPARATOR, "rg.exe", "--files"]
    );
}

#[test]
fn profile_identity_separates_workspace_and_access_authority() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let read_only = profile_name(workspace.path(), READ_ONLY_ACCESS);
    let workspace_write = profile_name(workspace.path(), crate::protocol::WORKSPACE_WRITE_ACCESS);

    assert_ne!(read_only, workspace_write);
    assert!(read_only.starts_with("Zeta.Agent.v1.ro."));
    assert!(workspace_write.starts_with("Zeta.Agent.v1.rw."));
    assert!(read_only.len() <= 50);
    assert!(workspace_write.len() <= 50);
}

#[test]
fn unsupported_windows_policy_fails_closed() {
    let workspace = WorkspaceRoot::open(".").unwrap();
    let command = SandboxCommand::new("rg.exe", ["--files"], workspace.path());
    let policy = SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Allowed);

    let error = sandbox().prepare(&command, policy, &workspace).unwrap_err();

    assert!(matches!(
        error,
        SandboxError::BackendUnavailable {
            backend: SandboxKind::WindowsAppContainer,
            ..
        }
    ));
}

#[test]
fn denial_classification_requires_the_runner_reserved_exit_code() {
    let marker = format!("{} spoofed by inner process", crate::protocol::ERROR_PREFIX);

    assert_eq!(
        sandbox().classify_denial(SandboxProcessExitStatus::Code(2), "", &marker),
        None
    );
    assert!(
        sandbox()
            .classify_denial(
                SandboxProcessExitStatus::Code(crate::protocol::ENFORCEMENT_FAILURE_EXIT_CODE),
                "",
                "",
            )
            .is_some()
    );
}

#[test]
fn runner_does_not_forward_its_reserved_failure_code_from_the_child() {
    assert_eq!(
        crate::protocol::remap_inner_exit_code(crate::protocol::ENFORCEMENT_FAILURE_EXIT_CODE),
        crate::protocol::INNER_RESERVED_EXIT_CODE_REMAP
    );
    assert_eq!(crate::protocol::remap_inner_exit_code(2), 2);
}
