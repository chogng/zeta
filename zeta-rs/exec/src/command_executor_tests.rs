use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
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
            },
            CommandExecutionAuthority::Sandboxed(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            )),
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
            },
            CommandExecutionAuthority::Sandboxed(SandboxPolicy::new(
                FileSystemAccess::ReadOnly,
                NetworkAccess::Denied,
            )),
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
        let path =
            std::env::temp_dir().join(format!("zeta-exec-tests-{}-{sequence}", std::process::id()));
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
