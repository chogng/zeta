use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use zeta_async_utils::CancellationSource;
use zeta_sandboxing::{PreparedCommand, SandboxError, SandboxKind};

struct AllowAll;

impl ApprovalPolicy for AllowAll {
    fn requirement_for(&self, _: &str) -> ApprovalRequirement {
        ApprovalRequirement::NotRequired
    }
}

struct ReplacingBackend;

impl SandboxBackend for ReplacingBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        assert_eq!(
            policy,
            SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Denied)
        );
        Ok(PreparedCommand::new(
            SandboxKind::Unrestricted,
            "/bin/sh",
            ["-c", "printf prepared-by-backend"],
            command.working_directory(),
        ))
    }
}

struct MissingSandboxLauncher;

struct PassThroughBackend;

impl SandboxBackend for PassThroughBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        _: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        Ok(PreparedCommand::unrestricted(command))
    }
}

impl SandboxBackend for MissingSandboxLauncher {
    fn kind(&self) -> SandboxKind {
        SandboxKind::MacosSeatbelt
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        _: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        Ok(PreparedCommand::new(
            SandboxKind::MacosSeatbelt,
            "/zeta-test/missing-sandbox-launcher",
            Vec::<String>::new(),
            command.working_directory(),
        ))
    }
}

#[test]
fn executor_spawns_only_the_command_prepared_by_the_sandbox_backend() {
    let workspace = TestWorkspace::new();
    let executor =
        CommandExecutor::new(workspace.root(), ReplacingBackend, AllowAll, test_limits());

    let outcome = executor
        .execute(
            CommandRequest {
                program: "must-not-run".into(),
                arguments: Vec::new(),
                working_directory: ".".into(),
                input: CommandInput::Closed,
            },
            CommandExecutionAuthority::Sandboxed(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            )),
            &CancellationSource::new().token(),
        )
        .unwrap();
    let CommandExecutionOutcome::Completed(output) = outcome else {
        panic!("unrestricted test backend should complete normally");
    };

    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.stdout, "prepared-by-backend");
}

#[test]
fn missing_sandbox_launcher_is_safe_to_retry_because_the_action_never_started() {
    let workspace = TestWorkspace::new();
    let executor = CommandExecutor::new(
        workspace.root(),
        MissingSandboxLauncher,
        AllowAll,
        test_limits(),
    );

    let outcome = executor
        .execute(
            CommandRequest {
                program: "must-not-run".into(),
                arguments: Vec::new(),
                working_directory: ".".into(),
                input: CommandInput::Closed,
            },
            CommandExecutionAuthority::Sandboxed(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            )),
            &CancellationSource::new().token(),
        )
        .unwrap();

    let CommandExecutionOutcome::SandboxDenied(denial) = outcome else {
        panic!("sandbox launcher spawn failure should be a structured denial");
    };
    assert_eq!(
        denial.replay_safety(),
        zeta_protocol::ToolReplaySafety::SafeToRetry
    );
    assert_eq!(
        denial.output().exit_status(),
        zeta_protocol::ProcessExitStatus::Terminated
    );
}

#[cfg(unix)]
#[test]
fn executor_returns_bounded_output_with_explicit_truncation_markers() {
    let workspace = TestWorkspace::new();
    let executor = CommandExecutor::new(
        workspace.root(),
        PassThroughBackend,
        AllowAll,
        ExecutionLimits {
            timeout: Duration::from_secs(3),
            max_output_bytes: 5,
        },
    );

    let outcome = executor
        .execute(
            CommandRequest {
                program: "/bin/sh".into(),
                arguments: vec!["-c".into(), "printf 123456789; printf abc >&2".into()],
                working_directory: ".".into(),
                input: CommandInput::Closed,
            },
            CommandExecutionAuthority::Unrestricted,
            &CancellationSource::new().token(),
        )
        .unwrap();
    let CommandExecutionOutcome::Completed(output) = outcome else {
        panic!("pass-through command should complete");
    };

    assert_eq!(output.stdout, "12345");
    assert!(output.stderr.is_empty());
    assert!(output.stdout_truncated);
    assert!(output.stderr_truncated);
}

#[cfg(unix)]
#[test]
fn executor_writes_explicit_bytes_to_child_stdin() {
    let workspace = TestWorkspace::new();
    let executor = CommandExecutor::new(
        workspace.root(),
        PassThroughBackend,
        AllowAll,
        test_limits(),
    );

    let outcome = executor
        .execute(
            CommandRequest {
                program: "/bin/sh".into(),
                arguments: vec!["-c".into(), "cat".into()],
                working_directory: ".".into(),
                input: CommandInput::Bytes(b"zeta-hook-payload".to_vec()),
            },
            CommandExecutionAuthority::Unrestricted,
            &CancellationSource::new().token(),
        )
        .unwrap();
    let CommandExecutionOutcome::Completed(output) = outcome else {
        panic!("pass-through command should complete");
    };

    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.stdout, "zeta-hook-payload");
}

#[cfg(unix)]
#[test]
fn executor_terminates_a_running_process_when_cancelled() {
    let workspace = TestWorkspace::new();
    let executor = CommandExecutor::new(
        workspace.root(),
        PassThroughBackend,
        AllowAll,
        test_limits(),
    );
    let cancellation = CancellationSource::new();
    let token = cancellation.token();
    let running = std::thread::spawn(move || {
        executor.execute(
            CommandRequest {
                program: "/bin/sleep".into(),
                arguments: vec!["10".into()],
                working_directory: ".".into(),
                input: CommandInput::Closed,
            },
            CommandExecutionAuthority::Unrestricted,
            &token,
        )
    });

    std::thread::sleep(Duration::from_millis(50));
    cancellation.cancel();
    let result = running.join().unwrap();
    assert!(matches!(
        result,
        Err(ExecutionError::CancelledAfterStart(_))
    ));
}

fn test_limits() -> ExecutionLimits {
    ExecutionLimits {
        timeout: Duration::from_secs(3),
        max_output_bytes: 16 * 1024,
    }
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-tool-executor-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn root(&self) -> WorkspaceRoot {
        WorkspaceRoot::open(&self.path).unwrap()
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
