use super::*;
use serde_json::json;
use std::fs;
use std::future::Future;
#[cfg(target_os = "macos")]
use std::net::TcpListener;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use zeta_async_utils::CancellationSource;
use zeta_protocol::{ToolCallId, TurnId};
use zeta_sandboxing::{
    PreparedCommand, SandboxBackend, SandboxCommand, SandboxError, SandboxKind, SandboxPolicy,
    WorkspaceRoot,
};
use zeta_tools::{
    ProcessExitStatus, ToolBinding, ToolBindingId, ToolDefinition, ToolEnvironmentId,
    ToolExecutionContext, ToolExecutionOutcome, ToolExecutor, ToolInvocation, ToolOperationId,
    ToolOutputStatus, ToolPayload, ToolRegistryGeneration, ToolReplaySafety, ToolRuntimeAuthority,
    ToolRuntimeKey,
};

struct DenyAll;

impl ApprovalPolicy for DenyAll {
    fn requirement_for(&self, _: &str) -> zeta_exec::ApprovalRequirement {
        zeta_exec::ApprovalRequirement::Denied
    }
}

struct AllowAll;

impl ApprovalPolicy for AllowAll {
    fn requirement_for(&self, _: &str) -> zeta_exec::ApprovalRequirement {
        zeta_exec::ApprovalRequirement::NotRequired
    }
}

struct MustNotPrepare;

impl SandboxBackend for MustNotPrepare {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        _: &SandboxCommand,
        _: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        panic!("denied commands must not reach sandbox preparation")
    }
}

struct RecordingBackend {
    policies: Arc<Mutex<Vec<SandboxPolicy>>>,
}

struct UnavailableBackend;

impl SandboxBackend for UnavailableBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::MacosSeatbelt
    }

    fn prepare(
        &self,
        _: &SandboxCommand,
        _: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        Err(SandboxError::BackendUnavailable {
            backend: SandboxKind::MacosSeatbelt,
            message: "test backend unavailable".into(),
        })
    }
}

impl SandboxBackend for RecordingBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        _: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        self.policies.lock().unwrap().push(policy);
        Ok(PreparedCommand::new(
            SandboxKind::Unrestricted,
            "/bin/sh",
            ["-c", "printf sandbox-authority-reached-executor"],
            command.working_directory(),
        ))
    }
}

#[test]
fn denied_command_is_returned_as_a_model_visible_error() {
    let workspace = TestWorkspace::new();
    let tool = ShellCommandTool::new(
        environment_id(),
        workspace.root(),
        MustNotPrepare,
        DenyAll,
        ShellCommandLimits {
            timeout: Duration::from_secs(1),
            max_output_bytes: 1024,
        },
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"program": "not-started", "arguments": [], "working_directory": "."}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("denied command should return a tool error");
    };
    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert!(format!("{:?}", output.content()).contains("denied by policy"));
}

#[test]
fn empty_program_is_rejected_before_execution() {
    let workspace = TestWorkspace::new();
    let tool = ShellCommandTool::new(
        environment_id(),
        workspace.root(),
        MustNotPrepare,
        DenyAll,
        ShellCommandLimits {
            timeout: Duration::from_secs(1),
            max_output_bytes: 1024,
        },
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"program": "", "arguments": [], "working_directory": "."}),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("empty program should return a tool error");
    };
    assert_eq!(output.status(), ToolOutputStatus::Error);
    assert!(format!("{:?}", output.content()).contains("must not be empty"));
}

#[test]
fn each_invocation_authority_reaches_the_process_backend() {
    let workspace = TestWorkspace::new();
    let policies = Arc::new(Mutex::new(Vec::new()));
    let sandbox = SandboxPolicy::new(
        zeta_sandboxing::FileSystemAccess::WorkspaceWrite,
        zeta_sandboxing::NetworkAccess::Denied,
    );
    let tool = ShellCommandTool::new(
        environment_id(),
        workspace.root(),
        RecordingBackend {
            policies: policies.clone(),
        },
        AllowAll,
        ShellCommandLimits {
            timeout: Duration::from_secs(1),
            max_output_bytes: 1024,
        },
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation_with_authority(
        &definition,
        json!({"program": "must-be-replaced", "arguments": [], "working_directory": "."}),
        ToolRuntimeAuthority::Sandboxed(sandbox),
    )));

    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("sandboxed command should return its backend-prepared output");
    };
    assert_eq!(output.status(), ToolOutputStatus::Success);
    assert!(format!("{:?}", output.content()).contains("sandbox-authority-reached-executor"));

    let outcome = resolve(tool.execute(invocation(
        &definition,
        json!({"program": "must-be-replaced", "arguments": [], "working_directory": "."}),
    )));
    let ToolExecutionOutcome::Returned(output) = outcome else {
        panic!("unrestricted command should return its backend-prepared output");
    };
    assert_eq!(output.status(), ToolOutputStatus::Success);
    assert_eq!(
        *policies.lock().unwrap(),
        vec![
            sandbox,
            SandboxPolicy::new(
                zeta_sandboxing::FileSystemAccess::FullAccess,
                zeta_sandboxing::NetworkAccess::Allowed,
            ),
        ]
    );
}

#[test]
fn unavailable_sandbox_is_a_safe_to_retry_structured_denial() {
    let workspace = TestWorkspace::new();
    let tool = ShellCommandTool::new(
        environment_id(),
        workspace.root(),
        UnavailableBackend,
        AllowAll,
        ShellCommandLimits {
            timeout: Duration::from_secs(1),
            max_output_bytes: 1024,
        },
    )
    .unwrap();
    let definition = tool.definition();

    let outcome = resolve(tool.execute(invocation_with_authority(
        &definition,
        json!({"program": "/usr/bin/true", "arguments": [], "working_directory": "."}),
        ToolRuntimeAuthority::Sandboxed(SandboxPolicy::new(
            zeta_sandboxing::FileSystemAccess::ReadOnly,
            zeta_sandboxing::NetworkAccess::Denied,
        )),
    )));

    let ToolExecutionOutcome::SandboxDenied(denial) = outcome else {
        panic!("unavailable sandbox should be a structured denial");
    };
    assert_eq!(denial.replay_safety(), ToolReplaySafety::SafeToRetry);
    assert_eq!(denial.output().exit_status(), ProcessExitStatus::Terminated);
    assert!(denial.output().stdout().is_empty());
    assert!(denial.output().stderr().is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn sandboxed_tool_invocation_enforces_workspace_metadata_and_network_boundaries() {
    use zeta_sandboxing::MacosSeatbeltSandbox;

    let workspace = TestWorkspace::new();
    let protected = workspace.path.join(".git");
    fs::create_dir_all(&protected).unwrap();
    let outside = workspace.path.with_extension("outside");
    let _outside_cleanup = RemovePathOnDrop(outside.clone());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let sandbox = SandboxPolicy::new(
        zeta_sandboxing::FileSystemAccess::WorkspaceWrite,
        zeta_sandboxing::NetworkAccess::Denied,
    );
    let tool = ShellCommandTool::new(
        environment_id(),
        workspace.root(),
        MacosSeatbeltSandbox::new(),
        AllowAll,
        ShellCommandLimits {
            timeout: Duration::from_secs(3),
            max_output_bytes: 16 * 1024,
        },
    )
    .unwrap();
    let definition = tool.definition();

    let allowed_outcome = execute_sandboxed_command(
        &tool,
        &definition,
        sandbox,
        "/usr/bin/touch",
        vec!["ordinary.txt".into()],
    );
    if let ToolExecutionOutcome::SandboxDenied(denial) = &allowed_outcome {
        assert_eq!(denial.replay_safety(), ToolReplaySafety::SafeToRetry);
        assert!(denial.reason().contains("could not apply"));
        return;
    }
    let allowed = completed_output(allowed_outcome);
    assert_eq!(allowed["result"]["exit_code"], json!(0));
    assert!(workspace.path.join("ordinary.txt").exists());

    let outside_write = execute_sandboxed_command(
        &tool,
        &definition,
        sandbox,
        "/usr/bin/touch",
        vec![outside.to_string_lossy().into_owned()],
    );
    assert_sandbox_denial(outside_write);
    assert!(!outside.exists());

    let metadata_write = execute_sandboxed_command(
        &tool,
        &definition,
        sandbox,
        "/usr/bin/touch",
        vec![".git/persisted-hook".into()],
    );
    assert_sandbox_denial(metadata_write);
    assert!(!protected.join("persisted-hook").exists());

    let network = execute_sandboxed_command(
        &tool,
        &definition,
        sandbox,
        "/usr/bin/nc",
        vec![
            "-z".into(),
            "-w".into(),
            "1".into(),
            "127.0.0.1".into(),
            port.to_string(),
        ],
    );
    assert_sandbox_denial(network);
}

#[cfg(target_os = "macos")]
fn execute_sandboxed_command(
    tool: &ShellCommandTool<AllowAll, zeta_sandboxing::MacosSeatbeltSandbox>,
    definition: &ToolDefinition,
    sandbox: SandboxPolicy,
    program: &str,
    arguments: Vec<String>,
) -> ToolExecutionOutcome {
    resolve(tool.execute(invocation_with_authority(
        definition,
        json!({
            "program": program,
            "arguments": arguments,
            "working_directory": ".",
        }),
        ToolRuntimeAuthority::Sandboxed(sandbox),
    )))
}

#[cfg(target_os = "macos")]
fn completed_output(outcome: ToolExecutionOutcome) -> serde_json::Value {
    let ToolExecutionOutcome::Returned(output) = &outcome else {
        panic!("allowed sandboxed command should return its terminal result: {outcome:?}");
    };
    assert_eq!(output.status(), ToolOutputStatus::Success);
    let [zeta_tools::ToolContent::Text(text)] = output.content() else {
        panic!("shell command should return one JSON text item");
    };
    serde_json::from_str(text).unwrap()
}

#[cfg(target_os = "macos")]
fn assert_sandbox_denial(outcome: ToolExecutionOutcome) {
    let ToolExecutionOutcome::SandboxDenied(denial) = outcome else {
        panic!("sandbox enforcement should return a structured denial");
    };
    assert_eq!(denial.replay_safety(), ToolReplaySafety::MayHaveSideEffects);
    assert!(matches!(
        denial.output().exit_status(),
        ProcessExitStatus::Code(code) if code != 0
    ));
    assert!(!denial.output().stderr().is_empty());
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-shell-command-tests-{}-{sequence}",
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

#[cfg(target_os = "macos")]
struct RemovePathOnDrop(PathBuf);

#[cfg(target_os = "macos")]
impl Drop for RemovePathOnDrop {
    fn drop(&mut self) {
        if self.0.is_dir() {
            let _ = fs::remove_dir_all(&self.0);
        } else {
            let _ = fs::remove_file(&self.0);
        }
    }
}

fn environment_id() -> ToolEnvironmentId {
    ToolEnvironmentId::new("test-environment").unwrap()
}

fn invocation(definition: &ToolDefinition, arguments: serde_json::Value) -> ToolInvocation {
    invocation_with_authority(definition, arguments, ToolRuntimeAuthority::Unrestricted)
}

fn invocation_with_authority(
    definition: &ToolDefinition,
    arguments: serde_json::Value,
    authority: ToolRuntimeAuthority,
) -> ToolInvocation {
    ToolInvocation::new(
        ToolOperationId::new("operation-1").unwrap(),
        ToolCallId::new("call-1").unwrap(),
        TurnId::new("turn-1").unwrap(),
        ToolBinding::new(
            ToolRegistryGeneration::new(1),
            ToolBindingId::new("binding-1").unwrap(),
            definition.name().clone(),
            definition.digest(),
            ToolRuntimeKey::new("local:test").unwrap(),
        ),
        ToolPayload::FunctionArguments(arguments),
        ToolExecutionContext::new(
            environment_id(),
            CancellationSource::new().token(),
            authority,
        ),
    )
}

fn resolve(
    mut future: Pin<Box<dyn Future<Output = ToolExecutionOutcome> + Send + '_>>,
) -> ToolExecutionOutcome {
    let waker: &Waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(outcome) => outcome,
        Poll::Pending => panic!("local tool future should complete synchronously"),
    }
}
