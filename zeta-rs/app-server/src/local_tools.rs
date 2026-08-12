use serde_json::json;
use std::fmt;
use std::path::{Component, Path};
use std::sync::Arc;
use std::time::Duration;
use zeta_apply_patch::ApplyPatchLimits;
use zeta_apply_patch::ApplyPatchTool;
use zeta_async_utils::CancellationToken;
use zeta_core::{
    CoreError, PolicyService, ToolAuthorization, ToolExecutionFacts, ToolOutputSink, ToolService,
};
use zeta_file_system::LocalFileSystem;
use zeta_file_system_tool::FileSystemLimits;
use zeta_file_system_tool::FileSystemTool;
use zeta_install_context::{ExecutableCandidates, InstallContext, ManagedExecutable};
use zeta_policy::{
    ActionDigest, ActionKind, ActionProvenance, ActionReviewPhase, ActionReviewRequest,
    ActionSource, ApprovalRequest, Capability, CapabilityKind, CapabilitySet, ExecutionDecision,
    PolicyRevision, ProcessInvocationKind, ResolvedAction, SandboxCompatibility,
};
use zeta_protocol::{ToolCall, ToolDefinition, ToolExecutionOutput, ToolOutputStream};
use zeta_sandboxing::{FileSystemAccess, NetworkAccess, SandboxBackend, SandboxPolicy};
use zeta_shell_command::{
    ApprovalPolicy, ApprovalRequirement, CommandExecutionAuthority, CommandExecutionOutcome,
    ExecutionError, RipgrepDiscoveryError, RipgrepExecutable, ShellCommandLimits,
    ShellCommandRequest, ShellCommandTool,
};
use zeta_tools::{ToolPayload, to_protocol_tool_definition};
use zeta_workspace::{TrustedWorkspace, WorkspaceCapability, WorkspaceRoot};

use crate::tool_composition::ToolCompositionError;
use crate::tool_composition::ToolPort;
use crate::tool_executor_adapter::PreparedToolExecution;
use crate::tool_executor_adapter::ToolExecutorReviewer;

mod suite;

pub(crate) use suite::LocalToolSuite;

pub(crate) const LOCAL_POLICY_REVISION: &str = "local-tools-v5";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_OUTPUT_BYTES: usize = 256 * 1024;

pub(crate) struct LocalToolComposition {
    pub(crate) tools: Arc<dyn ToolService>,
    pub(crate) policy: Arc<dyn PolicyService>,
    pub(crate) ripgrep: RipgrepExecutable,
    executors: Vec<LocalExecutorContribution>,
}

struct LocalExecutorContribution {
    executor: Arc<dyn zeta_tools::ToolExecutor>,
    environment_id: zeta_tools::ToolEnvironmentId,
    reviewer: Arc<dyn ToolExecutorReviewer>,
}

pub(crate) fn compose_local_tools(
    workspace: TrustedWorkspace,
) -> Result<LocalToolComposition, LocalToolError> {
    if workspace.capability() != WorkspaceCapability::ExecuteProcess {
        return Err(LocalToolError::trust(
            "local tools require the execute-process Workspace capability",
        ));
    }
    let install_context = InstallContext::current();
    let ripgrep = resolve_ripgrep(&install_context).map_err(LocalToolError::ripgrep)?;
    let environment_id = zeta_tools::ToolEnvironmentId::new("local-workspace")
        .map_err(LocalToolError::definition)?;
    let reviewer: Arc<dyn ToolExecutorReviewer> = Arc::new(LocalExecutorReviewer {
        workspace: workspace.clone(),
        ripgrep: ripgrep.clone(),
    });
    let shell_executor: Arc<dyn zeta_tools::ToolExecutor> = Arc::new(
        ShellCommandTool::new(
            environment_id.clone(),
            workspace.root().clone(),
            native_sandbox(&install_context)?,
            CoreAuthorized,
            ShellCommandLimits {
                timeout: DEFAULT_TIMEOUT,
                max_output_bytes: DEFAULT_OUTPUT_BYTES,
            },
        )
        .map_err(LocalToolError::definition)?,
    );
    let file_system_executor: Arc<dyn zeta_tools::ToolExecutor> = Arc::new(
        FileSystemTool::new(
            environment_id.clone(),
            Arc::new(LocalFileSystem::new(workspace.root().clone())),
            FileSystemLimits::default(),
        )
        .map_err(LocalToolError::definition)?,
    );
    let apply_patch_executor: Arc<dyn zeta_tools::ToolExecutor> = Arc::new(
        ApplyPatchTool::new(
            environment_id.clone(),
            workspace.root().clone(),
            ApplyPatchLimits::default(),
        )
        .map_err(LocalToolError::definition)?,
    );
    let policy = LocalShellPolicy;
    let shell = LocalShellToolService::new(
        workspace.clone(),
        ripgrep.clone(),
        native_sandbox(&install_context)?,
    )?;
    let service = LocalToolSuite::new(shell, ripgrep.clone());
    Ok(LocalToolComposition {
        tools: Arc::new(service),
        policy: Arc::new(policy),
        ripgrep,
        executors: vec![
            LocalExecutorContribution {
                executor: shell_executor,
                environment_id: environment_id.clone(),
                reviewer: Arc::clone(&reviewer),
            },
            LocalExecutorContribution {
                executor: file_system_executor,
                environment_id: environment_id.clone(),
                reviewer: Arc::clone(&reviewer),
            },
            LocalExecutorContribution {
                executor: apply_patch_executor,
                environment_id,
                reviewer,
            },
        ],
    })
}

impl LocalToolComposition {
    #[cfg(test)]
    pub(crate) fn without_executors(
        tools: Arc<dyn ToolService>,
        policy: Arc<dyn PolicyService>,
        ripgrep: RipgrepExecutable,
    ) -> Self {
        Self {
            tools,
            policy,
            ripgrep,
            executors: Vec::new(),
        }
    }

    pub(crate) fn tool_port(&self) -> Result<ToolPort, ToolCompositionError> {
        let mut port = ToolPort::local(Arc::clone(&self.tools), Arc::clone(&self.policy));
        let local_definitions = self.tools.definitions();
        for hidden in ["shell-command", "read_file", "write_file", "edit"] {
            let name =
                zeta_protocol::ToolName::new(hidden).expect("static local tool name is valid");
            if local_definitions
                .iter()
                .any(|definition| definition.name == name)
            {
                port = port.with_tool_exposure(&name, zeta_tools::ToolExposure::Hidden)?;
            }
        }
        for contribution in &self.executors {
            port = port.with_executor(
                Arc::clone(&contribution.executor),
                contribution.environment_id.clone(),
                Arc::clone(&contribution.reviewer),
            )?;
        }
        Ok(port)
    }
}

pub(crate) fn append_local_tool(
    composition: LocalToolComposition,
    tool: Arc<dyn ToolService>,
) -> LocalToolComposition {
    LocalToolComposition {
        tools: Arc::new(ExtendedLocalTools {
            primary: composition.tools,
            extension: tool,
        }),
        policy: composition.policy,
        ripgrep: composition.ripgrep,
        executors: composition.executors,
    }
}

struct ExtendedLocalTools {
    primary: Arc<dyn ToolService>,
    extension: Arc<dyn ToolService>,
}

impl ToolService for ExtendedLocalTools {
    fn definitions(&self) -> Vec<ToolDefinition> {
        let mut definitions = self.primary.definitions();
        definitions.extend(self.extension.definitions());
        definitions
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        if self
            .extension
            .definitions()
            .iter()
            .any(|definition| definition.name == call.name)
        {
            self.extension.prepare(call)
        } else {
            self.primary.prepare(call)
        }
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        if self
            .extension
            .definitions()
            .iter()
            .any(|definition| definition.name == call.name)
        {
            self.extension.execute(call, authorization, cancellation)
        } else {
            self.primary.execute(call, authorization, cancellation)
        }
    }

    fn execute_streaming(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        if self
            .extension
            .definitions()
            .iter()
            .any(|definition| definition.name == call.name)
        {
            self.extension
                .execute_streaming(call, authorization, cancellation, sink)
        } else {
            self.primary
                .execute_streaming(call, authorization, cancellation, sink)
        }
    }

    fn execute_with_facts(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
    ) -> Result<ToolExecutionOutput, CoreError> {
        if self
            .extension
            .definitions()
            .iter()
            .any(|definition| definition.name == call.name)
        {
            self.extension
                .execute_with_facts(call, authorization, cancellation, facts)
        } else {
            self.primary
                .execute_with_facts(call, authorization, cancellation, facts)
        }
    }

    fn execute_streaming_with_facts(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        if self
            .extension
            .definitions()
            .iter()
            .any(|definition| definition.name == call.name)
        {
            self.extension.execute_streaming_with_facts(
                call,
                authorization,
                cancellation,
                facts,
                sink,
            )
        } else {
            self.primary.execute_streaming_with_facts(
                call,
                authorization,
                cancellation,
                facts,
                sink,
            )
        }
    }
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
    workspace: TrustedWorkspace,
    ripgrep: RipgrepExecutable,
    shell: ShellCommandTool<CoreAuthorized, B>,
    definition: ToolDefinition,
}

impl<B: SandboxBackend> LocalShellToolService<B> {
    fn new(
        workspace: TrustedWorkspace,
        ripgrep: RipgrepExecutable,
        backend: B,
    ) -> Result<Self, LocalToolError> {
        let shell = ShellCommandTool::new(
            zeta_tools::ToolEnvironmentId::new("local-workspace")
                .map_err(LocalToolError::definition)?,
            workspace.root().clone(),
            backend,
            CoreAuthorized,
            ShellCommandLimits {
                timeout: DEFAULT_TIMEOUT,
                max_output_bytes: DEFAULT_OUTPUT_BYTES,
            },
        )
        .map_err(LocalToolError::definition)?;
        let definition = to_protocol_tool_definition(shell.host_definition())
            .map_err(LocalToolError::definition)?;
        Ok(Self {
            workspace,
            ripgrep,
            shell,
            definition,
        })
    }

    fn materialize(&self, call: &ToolCall) -> Result<ShellCommandRequest, CoreError> {
        self.workspace
            .ensure_active()
            .map_err(|error| CoreError::Policy(error.to_string()))?;
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
        if request.program() == "rg" {
            validate_workspace_arguments(self.workspace.root(), &request)?;
        }
        let working_directory = self
            .workspace
            .root()
            .resolve_existing(request.working_directory())
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        if !working_directory.is_dir() {
            return Err(CoreError::Policy(
                "shell-command working directory must be a directory".into(),
            ));
        }
        if request.program() == "rg" {
            self.ripgrep
                .materialize(request)
                .map_err(|error| CoreError::Policy(error.to_string()))
        } else {
            Ok(request)
        }
    }

    fn review_request(
        &self,
        request: &ShellCommandRequest,
    ) -> Result<ActionReviewRequest, CoreError> {
        let canonical_working_directory = self
            .workspace
            .root()
            .resolve_existing(request.working_directory())
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        let canonical = serde_json::to_vec(&json!({
            "program": request.program(),
            "arguments": request.arguments(),
            "working_directory": canonical_working_directory,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        let is_ripgrep = request.program() == self.ripgrep.path().to_string_lossy();
        let capabilities = if is_ripgrep {
            local_capabilities(self.workspace.root(), &self.ripgrep)
        } else {
            shell_capabilities()
        };
        let sandbox = if is_ripgrep {
            read_only_sandbox()
        } else {
            shell_sandbox()
        };
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::LocalProcess(if is_ripgrep {
                    ProcessInvocationKind::Direct
                } else {
                    ProcessInvocationKind::Shell
                }),
                format!(
                    "run {} in {}",
                    request.program(),
                    canonical_working_directory.display()
                ),
                capabilities,
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
            SandboxCompatibility::Supported(sandbox),
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
            | ToolAuthorization::PermissionBypassed(_)
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
                    "shell command was cancelled after process start: {reason}"
                )))
            }
            Err(ExecutionError::TimedOut) => Ok(ToolExecutionOutput::OutcomeUnknown(
                "shell command timed out after process start".into(),
            )),
            Err(error) => Ok(ToolExecutionOutput::Failure(execution_error(error))),
        }
    }

    fn execute_streaming(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let output = self.execute(call, authorization, cancellation)?;
        if let ToolExecutionOutput::Success(text) = &output
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
        {
            if let Some(stdout) = value.get("stdout").and_then(serde_json::Value::as_str) {
                sink.emit(ToolOutputStream::Stdout, stdout.to_owned())?;
            }
            if let Some(stderr) = value.get("stderr").and_then(serde_json::Value::as_str) {
                sink.emit(ToolOutputStream::Stderr, stderr.to_owned())?;
            }
        }
        Ok(output)
    }
}

struct LocalExecutorReviewer {
    workspace: TrustedWorkspace,
    ripgrep: RipgrepExecutable,
}

impl ToolExecutorReviewer for LocalExecutorReviewer {
    fn prepare(&self, call: &ToolCall) -> Result<PreparedToolExecution, CoreError> {
        self.workspace
            .ensure_active()
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        if call.name.as_str() == "shell-command" {
            let (review, request) = self.prepare_shell(call)?;
            return Ok(PreparedToolExecution::new(
                review,
                ToolPayload::FunctionArguments(json!({
                    "program": request.program(),
                    "arguments": request.arguments(),
                    "working_directory": request.working_directory(),
                })),
            ));
        }
        let review = match call.name.as_str() {
            "file-system" => self.prepare_file_system(call),
            "apply-patch" => self.prepare_apply_patch(call),
            _ => Err(CoreError::Policy(format!(
                "local executor reviewer does not own tool {}",
                call.name
            ))),
        }?;
        Ok(PreparedToolExecution::new(
            review,
            ToolPayload::FunctionArguments(call.arguments.clone()),
        ))
    }
}

impl LocalExecutorReviewer {
    fn prepare_shell(
        &self,
        call: &ToolCall,
    ) -> Result<(ActionReviewRequest, ShellCommandRequest), CoreError> {
        let mut request = ShellCommandRequest::from_arguments(&ToolPayload::FunctionArguments(
            call.arguments.clone(),
        ))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        if request.program() == "rg" {
            validate_workspace_arguments(self.workspace.root(), &request)?;
        }
        let working_directory = self
            .workspace
            .root()
            .resolve_existing(request.working_directory())
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        if !working_directory.is_dir() {
            return Err(CoreError::Policy(
                "shell-command working directory must be a directory".into(),
            ));
        }
        if request.program() == "rg" {
            request = self
                .ripgrep
                .materialize(request)
                .map_err(|error| CoreError::Policy(error.to_string()))?;
        }
        let canonical = serde_json::to_vec(&json!({
            "program": request.program(),
            "arguments": request.arguments(),
            "working_directory": working_directory,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        let is_ripgrep = request.program() == self.ripgrep.path().to_string_lossy();
        let capabilities = if is_ripgrep {
            local_capabilities(self.workspace.root(), &self.ripgrep)
        } else {
            shell_capabilities()
        };
        let sandbox = if is_ripgrep {
            read_only_sandbox()
        } else {
            shell_sandbox()
        };
        let review = ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::LocalProcess(if is_ripgrep {
                    ProcessInvocationKind::Direct
                } else {
                    ProcessInvocationKind::Shell
                }),
                format!(
                    "run {} in {}",
                    request.program(),
                    working_directory.display()
                ),
                capabilities,
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
            SandboxCompatibility::Supported(sandbox),
            PolicyRevision::new(LOCAL_POLICY_REVISION),
        );
        Ok((review, request))
    }

    fn prepare_file_system(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let operation = call
            .arguments
            .get("operation")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Policy("file-system operation must be a string".into()))?;
        if !matches!(operation, "read" | "list" | "metadata") {
            return Err(CoreError::Policy(format!(
                "unsupported file-system operation: {operation}"
            )));
        }
        let path = call
            .arguments
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Policy("file-system path must be a string".into()))?;
        let relative = if path.is_empty() { "." } else { path };
        let resolved = self
            .workspace
            .root()
            .resolve_existing(relative)
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        let canonical = serde_json::to_vec(&json!({
            "tool": call.name,
            "operation": operation,
            "path": resolved,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::SystemOperation,
                format!("{operation} {}", resolved.display()),
                CapabilitySet::new([Capability::new(
                    CapabilityKind::FileRead,
                    resolved.display().to_string(),
                )]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "file-system"),
            SandboxCompatibility::NotApplicable {
                reason: "the in-process file-system executor is confined by WorkspaceRoot".into(),
            },
            PolicyRevision::new(LOCAL_POLICY_REVISION),
        ))
    }

    fn prepare_apply_patch(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let patch = call
            .arguments
            .get("patch")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Policy("apply-patch patch must be a string".into()))?;
        let targets = materialize_patch_targets(self.workspace.root(), patch)?;
        let capabilities = targets.iter().flat_map(|target| {
            [
                Capability::new(CapabilityKind::FileRead, target.clone()),
                Capability::new(CapabilityKind::FileWrite, target.clone()),
            ]
        });
        let canonical = serde_json::to_vec(&json!({
            "tool": call.name,
            "patch": patch,
            "targets": &targets,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::FileSystemMutation,
                format!("apply patch to {} workspace file(s)", targets.len()),
                CapabilitySet::new(capabilities),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "apply-patch"),
            SandboxCompatibility::NotApplicable {
                reason: "apply-patch validates every target through WorkspaceRoot and commits host-mediated file mutations".into(),
            },
            PolicyRevision::new(LOCAL_POLICY_REVISION),
        ))
    }
}

fn materialize_patch_targets(
    workspace: &WorkspaceRoot,
    patch: &str,
) -> Result<Vec<String>, CoreError> {
    let mut targets = Vec::new();
    for line in patch.lines() {
        let operation = [
            ("*** Update File: ", true),
            ("*** Delete File: ", true),
            ("*** Add File: ", false),
        ]
        .into_iter()
        .find_map(|(prefix, existing)| line.strip_prefix(prefix).map(|path| (path, existing)));
        let Some((path, existing)) = operation else {
            continue;
        };
        if path.trim().is_empty() {
            return Err(CoreError::Policy(
                "apply-patch contains an empty target path".into(),
            ));
        }
        let resolved = if existing {
            workspace.resolve_existing(path)
        } else {
            workspace.resolve_for_write(path)
        }
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        targets.push(resolved.display().to_string());
    }
    targets.sort();
    targets.dedup();
    if targets.is_empty() {
        return Err(CoreError::Policy(
            "apply-patch contains no file operations to review".into(),
        ));
    }
    Ok(targets)
}

struct LocalShellPolicy;

impl PolicyService for LocalShellPolicy {
    fn revision(&self) -> String {
        LOCAL_POLICY_REVISION.into()
    }

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
            || !matches!(
                request.provenance().source_id(),
                "shell-command"
                    | "file-system"
                    | "apply-patch"
                    | "read_file"
                    | "write_file"
                    | "edit"
                    | "grep"
                    | "glob"
                    | crate::code_retrieval_tool::CODE_RETRIEVAL_TOOL_NAME
                    | crate::server::multi_agent_tools::SPAWN_AGENT_TOOL_NAME
                    | crate::server::multi_agent_tools::SEND_AGENT_MESSAGE_TOOL_NAME
                    | crate::server::multi_agent_tools::WAIT_AGENT_TOOL_NAME
            )
            || !matches!(
                request.action().kind(),
                ActionKind::LocalProcess(_)
                    | ActionKind::FileSystemMutation
                    | ActionKind::SystemOperation
            )
        {
            return Err(CoreError::Policy(
                "local shell policy rejected an action outside its exact review contract".into(),
            ));
        }
        match (request.phase(), request.sandbox()) {
            (ActionReviewPhase::Initial, SandboxCompatibility::Supported(policy))
                if *policy == read_only_sandbox() || *policy == shell_sandbox() =>
            {
                Ok(ExecutionDecision::RunSandboxed(*policy))
            }
            (ActionReviewPhase::Initial, SandboxCompatibility::NotApplicable { .. }) => {
                if request.provenance().source_id() == "file-system" {
                    return Ok(ExecutionDecision::RunUnsandboxed {
                        grant_id: zeta_policy::GrantId::new("local-file-system-read-only"),
                    });
                }
                if request.provenance().source_id()
                    == crate::code_retrieval_tool::CODE_RETRIEVAL_TOOL_NAME
                {
                    return Ok(ExecutionDecision::RunUnsandboxed {
                        grant_id: zeta_policy::GrantId::new("workspace-code-index-read-only"),
                    });
                }
                if matches!(
                    request.provenance().source_id(),
                    crate::server::multi_agent_tools::SPAWN_AGENT_TOOL_NAME
                        | crate::server::multi_agent_tools::SEND_AGENT_MESSAGE_TOOL_NAME
                        | crate::server::multi_agent_tools::WAIT_AGENT_TOOL_NAME
                ) {
                    return Ok(ExecutionDecision::RunUnsandboxed {
                        grant_id: zeta_policy::GrantId::new(format!(
                            "built-in:{}",
                            request.provenance().source_id()
                        )),
                    });
                }
                Ok(ExecutionDecision::AskUser(ApprovalRequest::new(
                    request.action().digest().clone(),
                    request.action().required_capabilities().clone(),
                    "the file mutation requires user approval",
                )))
            }
            (ActionReviewPhase::SandboxDenial(_), SandboxCompatibility::Supported(_)) => {
                Ok(ExecutionDecision::AskUser(ApprovalRequest::new(
                    request.action().digest().clone(),
                    request.action().required_capabilities().clone(),
                    "the command requires authority outside the workspace sandbox",
                )))
            }
            _ => Err(CoreError::Policy(
                "local shell review phase is invalid".into(),
            )),
        }
    }
}

fn local_capabilities(workspace: &WorkspaceRoot, ripgrep: &RipgrepExecutable) -> CapabilitySet {
    CapabilitySet::new([
        Capability::new(
            CapabilityKind::FileRead,
            workspace.canonical_path().to_string_lossy(),
        ),
        Capability::new(
            CapabilityKind::ProcessSpawn,
            ripgrep.path().to_string_lossy(),
        ),
    ])
}

fn shell_capabilities() -> CapabilitySet {
    CapabilitySet::new([
        Capability::new(CapabilityKind::FileRead, "/"),
        Capability::new(CapabilityKind::FileWrite, "/"),
        Capability::new(CapabilityKind::ProcessSpawn, "*"),
        Capability::new(CapabilityKind::Network, "*"),
        Capability::new(CapabilityKind::CredentialUse, "*"),
        Capability::new(CapabilityKind::ExternalMutation, "*"),
        Capability::new(CapabilityKind::SystemConfiguration, "*"),
        Capability::new(CapabilityKind::UserInterface, "*"),
    ])
}

fn read_only_sandbox() -> SandboxPolicy {
    SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Denied)
}

fn shell_sandbox() -> SandboxPolicy {
    SandboxPolicy::new(FileSystemAccess::WorkspaceWrite, NetworkAccess::Denied)
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
        if workspace
            .canonical_path()
            .join(&workspace_relative)
            .exists()
        {
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
        ExecutionError::ApprovalRequired => {
            "shell command unexpectedly requires host approval".into()
        }
        ExecutionError::Denied => "shell command was denied by the execution policy".into(),
        ExecutionError::Spawn(reason) => format!("could not execute shell command: {reason}"),
        ExecutionError::CancelledBeforeStart(reason) => {
            format!("ripgrep was cancelled before process start: {reason}")
        }
        ExecutionError::CancelledAfterStart(reason) => {
            format!("ripgrep was cancelled after process start: {reason}")
        }
        ExecutionError::TimedOut => "shell command timed out".into(),
        ExecutionError::Sandbox(error) => format!("shell command sandbox failed: {error}"),
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
    fn trust(error: impl fmt::Display) -> Self {
        Self(format!("workspace trust rejected local tools: {error}"))
    }

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
