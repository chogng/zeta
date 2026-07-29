use serde_json::json;
use std::fmt;
use std::path::{Component, Path};
use std::sync::Arc;
use std::time::Duration;
use zeta_async_utils::CancellationToken;
use zeta_core::{CoreError, PolicyService, ToolAuthorization, ToolService};
use zeta_install_context::{ExecutableCandidates, InstallContext, ManagedExecutable};
use zeta_policy::{
    ActionDigest, ActionKind, ActionProvenance, ActionReviewPhase, ActionReviewRequest,
    ActionSource, Capability, CapabilityKind, CapabilitySet, ExecutionDecision, PolicyRevision,
    ProcessInvocationKind, ResolvedAction, SandboxCompatibility,
};
use zeta_protocol::{ToolCall, ToolDefinition, ToolExecutionOutput};
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, SandboxBackend, SandboxPolicy, WorkspaceRoot,
};
use zeta_shell_command::{
    ApprovalPolicy, ApprovalRequirement, CommandExecutionAuthority, CommandExecutionOutcome,
    ExecutionError, RipgrepDiscoveryError, RipgrepExecutable, ShellCommandLimits,
    ShellCommandRequest, ShellCommandTool,
};
use zeta_tools::{ToolPayload, to_protocol_tool_definition};

const LOCAL_POLICY_REVISION: &str = "local-read-only-rg-v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_OUTPUT_BYTES: usize = 256 * 1024;

pub(crate) struct LocalToolComposition {
    pub(crate) tools: Arc<dyn ToolService>,
    pub(crate) policy: Arc<dyn PolicyService>,
    pub(crate) ripgrep: RipgrepExecutable,
}

pub(crate) fn compose_local_tools(
    workspace: WorkspaceRoot,
) -> Result<LocalToolComposition, LocalToolError> {
    let install_context = InstallContext::current();
    let ripgrep = resolve_ripgrep(&install_context).map_err(LocalToolError::ripgrep)?;
    let policy = LocalReadOnlyPolicy::new(&workspace, &ripgrep);
    let service = LocalShellToolService::new(
        workspace,
        ripgrep.clone(),
        native_sandbox(&install_context)?,
    )?;
    Ok(LocalToolComposition {
        tools: Arc::new(service),
        policy: Arc::new(policy),
        ripgrep,
    })
}

fn resolve_ripgrep(context: &InstallContext) -> Result<RipgrepExecutable, RipgrepDiscoveryError> {
    match context.executable_candidates(ManagedExecutable::Ripgrep) {
        ExecutableCandidates::ExplicitOverride(explicit_override) => {
            RipgrepExecutable::from_override(explicit_override.variable(), explicit_override.path())
        }
        ExecutableCandidates::SearchPaths(paths) => RipgrepExecutable::discover_candidates(paths),
    }
}

struct CoreAuthorized;

impl ApprovalPolicy for CoreAuthorized {
    fn requirement_for(&self, _: &str) -> ApprovalRequirement {
        ApprovalRequirement::NotRequired
    }
}

struct LocalShellToolService<B> {
    workspace: WorkspaceRoot,
    ripgrep: RipgrepExecutable,
    shell: ShellCommandTool<CoreAuthorized, B>,
    definition: ToolDefinition,
}

impl<B: SandboxBackend> LocalShellToolService<B> {
    fn new(
        workspace: WorkspaceRoot,
        ripgrep: RipgrepExecutable,
        backend: B,
    ) -> Result<Self, LocalToolError> {
        let shell = ShellCommandTool::new(
            zeta_tools::ToolEnvironmentId::new("local-workspace")
                .map_err(LocalToolError::definition)?,
            workspace.clone(),
            backend,
            CoreAuthorized,
            ShellCommandLimits {
                timeout: DEFAULT_TIMEOUT,
                max_output_bytes: DEFAULT_OUTPUT_BYTES,
            },
        )
        .map_err(LocalToolError::definition)?;
        let mut definition = to_protocol_tool_definition(shell.host_definition())
            .map_err(LocalToolError::definition)?;
        definition.description =
            "Search workspace file contents and paths with read-only ripgrep (`rg`).".into();
        if let Some(program) = definition
            .parameters
            .get_mut("properties")
            .and_then(|properties| properties.get_mut("program"))
        {
            *program = json!({
                "type": "string",
                "enum": ["rg"],
                "description": "The read-only ripgrep program."
            });
        }
        Ok(Self {
            workspace,
            ripgrep,
            shell,
            definition,
        })
    }

    fn materialize(&self, call: &ToolCall) -> Result<ShellCommandRequest, CoreError> {
        if call.name != self.definition.name {
            return Err(CoreError::Policy(format!(
                "tool is not available: {}",
                call.name
            )));
        }
        let request = ShellCommandRequest::from_arguments(&ToolPayload::FunctionArguments(
            call.arguments.clone(),
        ))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        validate_workspace_arguments(&self.workspace, &request)?;
        let working_directory = self
            .workspace
            .resolve_existing(request.working_directory())
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        if !working_directory.is_dir() {
            return Err(CoreError::Policy(
                "shell-command working directory must be a directory".into(),
            ));
        }
        self.ripgrep
            .materialize(request)
            .map_err(|error| CoreError::Policy(error.to_string()))
    }

    fn review_request(
        &self,
        request: &ShellCommandRequest,
    ) -> Result<ActionReviewRequest, CoreError> {
        let canonical_working_directory = self
            .workspace
            .resolve_existing(request.working_directory())
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        let canonical = serde_json::to_vec(&json!({
            "program": request.program(),
            "arguments": request.arguments(),
            "working_directory": canonical_working_directory,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        let capabilities = local_capabilities(&self.workspace, &self.ripgrep);
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::LocalProcess(ProcessInvocationKind::Direct),
                format!(
                    "run read-only ripgrep in {}",
                    canonical_working_directory.display()
                ),
                capabilities,
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
            SandboxCompatibility::Supported(read_only_sandbox()),
            PolicyRevision::new(LOCAL_POLICY_REVISION),
        ))
    }
}

impl<B: SandboxBackend> ToolService for LocalShellToolService<B> {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![self.definition.clone()]
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        self.review_request(&self.materialize(call)?)
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let request = self.materialize(call)?;
        let authority = match authorization {
            ToolAuthorization::Sandboxed(policy) => CommandExecutionAuthority::Sandboxed(*policy),
            ToolAuthorization::UnsandboxedGrant { .. }
            | ToolAuthorization::AutoReviewed(_)
            | ToolAuthorization::ApprovedOnce(_) => CommandExecutionAuthority::Unrestricted,
        };
        match self
            .shell
            .execute_authorized(request, authority, cancellation)
        {
            Ok(CommandExecutionOutcome::Completed(output)) => {
                let text = serde_json::to_string_pretty(&json!({
                    "exit_code": output.exit_code,
                    "stdout": output.stdout,
                    "stderr": output.stderr,
                    "stdout_truncated": output.stdout_truncated,
                    "stderr_truncated": output.stderr_truncated,
                }))
                .map_err(|error| CoreError::Execution(error.to_string()))?;
                Ok(ToolExecutionOutput::Success(text))
            }
            Ok(CommandExecutionOutcome::SandboxDenied(denial)) => {
                Ok(ToolExecutionOutput::SandboxDenied(denial))
            }
            Err(ExecutionError::CancelledAfterStart(reason)) => {
                Ok(ToolExecutionOutput::OutcomeUnknown(format!(
                    "ripgrep was cancelled after process start: {reason}"
                )))
            }
            Err(ExecutionError::TimedOut) => Ok(ToolExecutionOutput::OutcomeUnknown(
                "ripgrep timed out after process start".into(),
            )),
            Err(error) => Ok(ToolExecutionOutput::Failure(execution_error(error))),
        }
    }
}

struct LocalReadOnlyPolicy {
    capabilities: CapabilitySet,
}

impl LocalReadOnlyPolicy {
    fn new(workspace: &WorkspaceRoot, ripgrep: &RipgrepExecutable) -> Self {
        Self {
            capabilities: local_capabilities(workspace, ripgrep),
        }
    }
}

impl PolicyService for LocalReadOnlyPolicy {
    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        if request.policy_revision().as_str() != LOCAL_POLICY_REVISION
            || request.provenance().source() != &ActionSource::BuiltInTool
            || request.provenance().source_id() != "shell-command"
            || request.action().kind() != &ActionKind::LocalProcess(ProcessInvocationKind::Direct)
            || !matches!(request.phase(), ActionReviewPhase::Initial)
            || request.action().required_capabilities() != &self.capabilities
        {
            return Err(CoreError::Policy(
                "local read-only policy rejected a non-ripgrep action".into(),
            ));
        }
        match request.sandbox() {
            SandboxCompatibility::Supported(policy) if *policy == read_only_sandbox() => {
                Ok(ExecutionDecision::RunSandboxed(*policy))
            }
            _ => Err(CoreError::Policy(
                "read-only ripgrep requires an enforceable local sandbox".into(),
            )),
        }
    }
}

fn local_capabilities(workspace: &WorkspaceRoot, ripgrep: &RipgrepExecutable) -> CapabilitySet {
    CapabilitySet::new([
        Capability::new(CapabilityKind::FileRead, workspace.path().to_string_lossy()),
        Capability::new(
            CapabilityKind::ProcessSpawn,
            ripgrep.path().to_string_lossy(),
        ),
    ])
}

fn read_only_sandbox() -> SandboxPolicy {
    SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Denied)
}

fn validate_workspace_arguments(
    workspace: &WorkspaceRoot,
    request: &ShellCommandRequest,
) -> Result<(), CoreError> {
    for argument in request.arguments() {
        let path = Path::new(argument);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(CoreError::Policy(format!(
                "ripgrep argument escapes the workspace: {argument}"
            )));
        }
        let workspace_relative = request.working_directory().join(path);
        if workspace.path().join(&workspace_relative).exists() {
            workspace
                .resolve_existing(&workspace_relative)
                .map_err(|_| {
                    CoreError::Policy(format!(
                        "ripgrep argument resolves outside the workspace: {argument}"
                    ))
                })?;
        }
    }
    Ok(())
}

fn execution_error(error: ExecutionError) -> String {
    match error {
        ExecutionError::ApprovalRequired => "ripgrep unexpectedly requires host approval".into(),
        ExecutionError::Denied => "ripgrep was denied by the execution policy".into(),
        ExecutionError::Spawn(reason) => format!("could not execute ripgrep: {reason}"),
        ExecutionError::CancelledBeforeStart(reason) => {
            format!("ripgrep was cancelled before process start: {reason}")
        }
        ExecutionError::CancelledAfterStart(reason) => {
            format!("ripgrep was cancelled after process start: {reason}")
        }
        ExecutionError::TimedOut => "ripgrep timed out".into(),
        ExecutionError::Sandbox(error) => format!("ripgrep sandbox failed: {error}"),
    }
}

#[cfg(target_os = "macos")]
type NativeSandbox = zeta_sandboxing::MacosSeatbeltSandbox;
#[cfg(target_os = "macos")]
fn native_sandbox(_: &InstallContext) -> Result<NativeSandbox, LocalToolError> {
    Ok(NativeSandbox::new())
}

#[cfg(target_os = "linux")]
type NativeSandbox = zeta_linux_sandbox::LinuxSandbox;
#[cfg(target_os = "linux")]
fn native_sandbox(context: &InstallContext) -> Result<NativeSandbox, LocalToolError> {
    NativeSandbox::discover(context).map_err(LocalToolError::sandbox)
}

#[cfg(target_os = "windows")]
type NativeSandbox = zeta_windows_sandbox::WindowsSandbox;
#[cfg(target_os = "windows")]
fn native_sandbox(context: &InstallContext) -> Result<NativeSandbox, LocalToolError> {
    NativeSandbox::discover(context).map_err(LocalToolError::sandbox)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!("local shell tools require a supported sandbox backend");

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocalToolError(String);

impl LocalToolError {
    fn ripgrep(error: impl fmt::Display) -> Self {
        Self(format!("could not resolve ripgrep: {error}"))
    }

    fn definition(error: impl fmt::Display) -> Self {
        Self(format!("could not construct local tool registry: {error}"))
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn sandbox(error: impl fmt::Display) -> Self {
        Self(format!("could not resolve platform sandbox: {error}"))
    }
}

impl fmt::Display for LocalToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for LocalToolError {}

#[cfg(test)]
#[path = "local_tools_tests.rs"]
mod tests;
