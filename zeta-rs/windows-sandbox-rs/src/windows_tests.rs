use super::*;
use crate::protocol::{
    ACCESS_FLAG, COMMAND_SEPARATOR, CWD_FLAG, DIR_FLAG, READ_ONLY_ACCESS, SETUP_HELPER_FLAG,
};

fn sandbox() -> WindowsSandbox {
    WindowsSandbox::new("zeta-command-runner.exe", "zeta-windows-sandbox-setup.exe")
}

#[test]
fn resolves_dir_and_network_authority_for_the_native_launcher() {
    let dir = Dir::open_local(".").unwrap();
    let policy = SandboxPolicy::new(FileSystemAccess::DirectoryWrite, NetworkAccess::Denied);

    let plan = sandbox().plan(policy, &dir);

    assert_eq!(plan.dir(), dir.canonical_path());
    assert_eq!(plan.file_system(), FileSystemAccess::DirectoryWrite);
    assert_eq!(plan.network(), NetworkAccess::Denied);
}

#[test]
fn prepares_appcontainer_runner_with_frozen_setup_and_inner_command() {
    let dir = Dir::open_local(".").unwrap();
    let command = SandboxCommand::new("rg.exe", ["--files"], dir.canonical_path());
    let policy = SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Denied);

    let prepared = sandbox().prepare(&command, policy, &dir).unwrap();

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
    assert!(arguments.contains(&DIR_FLAG.to_owned()));
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
fn profile_identity_separates_dir_and_access_authority() {
    let dir = Dir::open_local(".").unwrap();
    let read_only = profile_name(dir.canonical_path(), READ_ONLY_ACCESS);
    let dir_write = profile_name(dir.canonical_path(), crate::protocol::DIR_WRITE_ACCESS);

    assert_ne!(read_only, dir_write);
    assert!(read_only.starts_with("Zeta.Agent.v1.ro."));
    assert!(dir_write.starts_with("Zeta.Agent.v1.rw."));
    assert!(read_only.len() <= 50);
    assert!(dir_write.len() <= 50);
}

#[test]
fn unsupported_windows_policy_fails_closed() {
    let dir = Dir::open_local(".").unwrap();
    let command = SandboxCommand::new("rg.exe", ["--files"], dir.canonical_path());
    let policy = SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Allowed);

    let error = sandbox().prepare(&command, policy, &dir).unwrap_err();

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
