use serde_json::json;
use std::fmt;
use std::path::{Component, Path};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use zeta_action_policy::{
    ActionClassifier, ActionDigest, ActionKind, ActionPolicyEngine, ActionPolicyRevision,
    ActionProvenance, ActionReviewRequest, ActionSource, AssessmentId, Capability, CapabilityKind,
    CapabilitySet, ClassifierAssessment, ClassifierRecommendation, ExecutionDecision,
    ProcessInvocationKind, ResolvedAction, ReviewFailurePolicy, SandboxCompatibility,
};
use zeta_apply_patch::ApplyPatchLimits;
use zeta_apply_patch::ApplyPatchTool;
use zeta_async_utils::CancellationToken;
use zeta_config::ResolvedConfig;
use zeta_config::UserExecPolicyConfig;
use zeta_config::WorkspaceExecPolicyConfig;
use zeta_config::WorkspaceId;
use zeta_core::{
    ActionPolicyService, CoreError, ToolAuthorization, ToolExecutionFacts, ToolOutputSink,
    ToolService,
};
use zeta_execpolicy::ExecPolicyActionKind;
use zeta_execpolicy::ExecPolicyDefault;
use zeta_execpolicy::ExecPolicyEffect;
use zeta_execpolicy::ExecPolicyLayer;
use zeta_execpolicy::ExecPolicyLayerId;
use zeta_execpolicy::ExecPolicyLayerKind;
use zeta_execpolicy::ExecPolicyRule;
use zeta_execpolicy::ExecPolicyRuleId;
use zeta_execpolicy::ExecPolicySelector;
use zeta_execpolicy::ExecPolicySnapshot;
use zeta_install_context::{ExecutableCandidates, InstallContext, ManagedExecutable};
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

#[path = "local_tools/additional_directories.rs"]
mod additional_directories;
mod suite;

pub(crate) use additional_directories::SessionAdditionalDirectoryAccess;
pub(crate) use suite::LocalToolSuite;

const LOCAL_GRANT_SNAPSHOT_REVISION: &str = "local-static-grants-v1";
const LOCAL_REVIEWER_POLICY_REVISION: &str = "local-interactive-review-v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_OUTPUT_BYTES: usize = 256 * 1024;

pub(crate) struct LocalToolComposition {
    pub(crate) tools: Arc<dyn ToolService>,
    pub(crate) policy: Arc<dyn ActionPolicyService>,
    pub(crate) ripgrep: RipgrepExecutable,
    action_policy_revision: ActionPolicyRevision,
    executors: Vec<LocalExecutorContribution>,
}

/// Durable configuration inputs used to compose one immutable local execution-policy snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LocalExecPolicyConfig {
    user: UserExecPolicyConfig,
    workspace: Option<(WorkspaceId, WorkspaceExecPolicyConfig)>,
}

impl LocalExecPolicyConfig {
    pub(crate) fn from_resolved(config: &ResolvedConfig) -> Self {
        Self {
            user: config.exec_policy.clone(),
            workspace: config.workspace.as_ref().map(|workspace| {
                (
                    workspace.workspace_id.clone(),
                    workspace.exec_policy.clone(),
                )
            }),
        }
    }

    fn snapshot(&self) -> Result<ExecPolicySnapshot, LocalToolError> {
        zeta_config::compose_exec_policy(
            ExecPolicyDefault::Deny("action is outside the local Tool policy contract".into()),
            vec![LOCAL_EXEC_POLICY_HOST_LAYER.clone()],
            &self.user,
            self.workspace
                .as_ref()
                .map(|(workspace_id, workspace)| (workspace_id, workspace)),
        )
        .map_err(LocalToolError::policy)
    }
}

struct LocalExecutorContribution {
    executor: Arc<dyn zeta_tools::ToolExecutor>,
    environment_id: zeta_tools::ToolEnvironmentId,
    reviewer: Arc<dyn ToolExecutorReviewer>,
}

pub(crate) fn compose_local_tools_with_config(
    workspace: TrustedWorkspace,
    policy_config: &LocalExecPolicyConfig,
    additional_directories: Arc<SessionAdditionalDirectoryAccess>,
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
    let exec_policy = policy_config.snapshot()?;
    let action_policy_revision = ActionPolicyRevision::from_components(
        exec_policy.revision(),
        LOCAL_GRANT_SNAPSHOT_REVISION,
        LOCAL_REVIEWER_POLICY_REVISION,
    );
    let reviewer: Arc<dyn ToolExecutorReviewer> = Arc::new(LocalExecutorReviewer {
        workspace: workspace.clone(),
        ripgrep: ripgrep.clone(),
        action_policy_revision: action_policy_revision.clone(),
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
    let apply_patch_executor: Arc<dyn zeta_tools::ToolExecutor> = Arc::new(
        ApplyPatchTool::new(
            environment_id.clone(),
            workspace.root().clone(),
            ApplyPatchLimits::default(),
        )
        .map_err(LocalToolError::definition)?,
    );
    let policy = LocalShellPolicy {
        exec_policy,
        action_policy_revision: action_policy_revision.clone(),
    };
    let shell = LocalShellToolService::new_with_action_policy_revision(
        workspace.clone(),
        ripgrep.clone(),
        native_sandbox(&install_context)?,
        action_policy_revision.clone(),
    )?;
    let service = LocalToolSuite::new(shell, ripgrep.clone(), additional_directories);
    Ok(LocalToolComposition {
        tools: Arc::new(service),
        policy: Arc::new(policy),
        ripgrep,
        action_policy_revision,
        executors: vec![
            LocalExecutorContribution {
                executor: shell_executor,
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
        policy: Arc<dyn ActionPolicyService>,
        ripgrep: RipgrepExecutable,
    ) -> Self {
        Self {
            tools,
            policy,
            ripgrep,
            action_policy_revision: local_policy_revision(),
            executors: Vec::new(),
        }
    }

    pub(crate) fn action_policy_revision(&self) -> &ActionPolicyRevision {
        &self.action_policy_revision
    }

    pub(crate) fn tool_port(&self) -> Result<ToolPort, ToolCompositionError> {
        let mut port = ToolPort::local(Arc::clone(&self.tools), Arc::clone(&self.policy));
        let local_definitions = self.tools.definitions();
        let shell_name =
            zeta_protocol::ToolName::new("shell-command").expect("static local tool name is valid");
        if local_definitions
            .iter()
            .any(|definition| definition.name == shell_name)
        {
            port = port.with_tool_exposure(&shell_name, zeta_tools::ToolExposure::Hidden)?;
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
        action_policy_revision: composition.action_policy_revision,
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

    fn prepare_with_facts(
        &self,
        call: &ToolCall,
        facts: &ToolExecutionFacts,
    ) -> Result<ActionReviewRequest, CoreError> {
        if self
            .extension
            .definitions()
            .iter()
            .any(|definition| definition.name == call.name)
        {
            self.extension.prepare_with_facts(call, facts)
        } else {
            self.primary.prepare_with_facts(call, facts)
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

    fn execute_streaming_with_facts_and_interactions(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
        interactions: Arc<dyn zeta_core::ToolInteractionService>,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        if self
            .extension
            .definitions()
            .iter()
            .any(|definition| definition.name == call.name)
        {
            self.extension
                .execute_streaming_with_facts_and_interactions(
                    call,
                    authorization,
                    cancellation,
                    facts,
                    interactions,
                    sink,
                )
        } else {
            self.primary.execute_streaming_with_facts_and_interactions(
                call,
                authorization,
                cancellation,
                facts,
                interactions,
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
    action_policy_revision: ActionPolicyRevision,
}

impl<B: SandboxBackend> LocalShellToolService<B> {
    #[cfg(test)]
    fn new(
        workspace: TrustedWorkspace,
        ripgrep: RipgrepExecutable,
        backend: B,
    ) -> Result<Self, LocalToolError> {
        Self::new_with_action_policy_revision(workspace, ripgrep, backend, local_policy_revision())
    }

    fn new_with_action_policy_revision(
        workspace: TrustedWorkspace,
        ripgrep: RipgrepExecutable,
        backend: B,
        action_policy_revision: ActionPolicyRevision,
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
            action_policy_revision,
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
            )
            .with_command(request.program().to_owned(), request.arguments().to_vec()),
            ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
            SandboxCompatibility::Supported(sandbox),
            self.action_policy_revision.clone(),
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
            | ToolAuthorization::ExecPolicyGranted(_)
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
    action_policy_revision: ActionPolicyRevision,
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
            "apply_patch" => self.prepare_apply_patch(call),
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
            )
            .with_command(request.program().to_owned(), request.arguments().to_vec()),
            ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
            SandboxCompatibility::Supported(sandbox),
            self.action_policy_revision.clone(),
        );
        Ok((review, request))
    }

    fn prepare_apply_patch(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let patch = call
            .arguments
            .get("patch")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Policy("apply_patch patch must be a string".into()))?;
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
            ActionProvenance::new(ActionSource::BuiltInTool, "apply_patch"),
            SandboxCompatibility::NotApplicable {
                reason: "apply_patch validates every target through WorkspaceRoot and commits host-mediated file mutations".into(),
            },
            self.action_policy_revision.clone(),
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
                "apply_patch contains an empty target path".into(),
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
            "apply_patch contains no file operations to review".into(),
        ));
    }
    Ok(targets)
}

struct LocalShellPolicy {
    exec_policy: ExecPolicySnapshot,
    action_policy_revision: ActionPolicyRevision,
}

impl Default for LocalShellPolicy {
    fn default() -> Self {
        let exec_policy = LocalExecPolicyConfig::default()
            .snapshot()
            .expect("static local execution policy is valid");
        Self {
            exec_policy,
            action_policy_revision: local_policy_revision(),
        }
    }
}

static LOCAL_EXEC_POLICY_HOST_LAYER: LazyLock<ExecPolicyLayer> = LazyLock::new(|| {
    let rules = [
        local_rule(
            "local-shell",
            "shell-command",
            ExecPolicyActionKind::LocalProcess,
            ExecPolicyEffect::Continue,
        ),
        local_rule(
            "local-read-file",
            "read_file",
            ExecPolicyActionKind::LocalProcess,
            ExecPolicyEffect::Continue,
        ),
        local_rule(
            "local-grep",
            "grep",
            ExecPolicyActionKind::LocalProcess,
            ExecPolicyEffect::Continue,
        ),
        local_rule(
            "local-glob",
            "glob",
            ExecPolicyActionKind::LocalProcess,
            ExecPolicyEffect::Continue,
        ),
        local_rule(
            "local-write-file",
            "write_file",
            ExecPolicyActionKind::FileSystemMutation,
            ExecPolicyEffect::Continue,
        ),
        local_rule(
            "local-edit",
            "edit",
            ExecPolicyActionKind::FileSystemMutation,
            ExecPolicyEffect::Continue,
        ),
        local_rule(
            "local-apply-patch",
            "apply_patch",
            ExecPolicyActionKind::FileSystemMutation,
            ExecPolicyEffect::Continue,
        ),
        local_rule(
            "workspace-code-index-read-only",
            crate::code_retrieval_tool::CODE_RETRIEVAL_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
        local_rule(
            "built-in:update_plan",
            crate::server::update_plan_tool::UPDATE_PLAN_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
        local_rule(
            "built-in:get_goal",
            crate::server::goal_tool::GET_GOAL_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
        local_rule(
            "built-in:create_goal",
            crate::server::goal_tool::CREATE_GOAL_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
        local_rule(
            "built-in:update_goal",
            crate::server::goal_tool::UPDATE_GOAL_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
        local_rule(
            "built-in:spawn_agent",
            crate::server::multi_agent_tools::SPAWN_AGENT_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
        local_rule(
            "built-in:send_agent_message",
            crate::server::multi_agent_tools::SEND_AGENT_MESSAGE_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
        local_rule(
            "built-in:wait_agent",
            crate::server::multi_agent_tools::WAIT_AGENT_TOOL_NAME,
            ExecPolicyActionKind::SystemOperation,
            ExecPolicyEffect::AllowUnsandboxed,
        ),
    ];
    ExecPolicyLayer::new(
        ExecPolicyLayerId::new("local-host"),
        ExecPolicyLayerKind::Host,
        rules,
    )
});

static LOCAL_ACTION_POLICY_REVISION: LazyLock<ActionPolicyRevision> = LazyLock::new(|| {
    let exec_policy = LocalExecPolicyConfig::default()
        .snapshot()
        .expect("static local execution policy is valid");
    ActionPolicyRevision::from_components(
        exec_policy.revision(),
        LOCAL_GRANT_SNAPSHOT_REVISION,
        LOCAL_REVIEWER_POLICY_REVISION,
    )
});

fn local_rule(
    rule_id: &str,
    source_id: &str,
    action_kind: ExecPolicyActionKind,
    effect: ExecPolicyEffect,
) -> ExecPolicyRule {
    ExecPolicyRule::new(
        ExecPolicyRuleId::new(rule_id),
        ExecPolicySelector::all([
            ExecPolicySelector::source(Some("built_in_tool".into()), Some(source_id.into())),
            ExecPolicySelector::ActionKind { action_kind },
        ]),
        effect,
    )
}

pub(crate) fn local_policy_revision() -> ActionPolicyRevision {
    LOCAL_ACTION_POLICY_REVISION.clone()
}

struct LocalPolicyClassifier;

impl ActionClassifier for LocalPolicyClassifier {
    type Error = std::convert::Infallible;

    fn classify(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ClassifierAssessment, Self::Error> {
        let _ = cancellation.check();
        let reason = if matches!(
            request.phase(),
            zeta_action_policy::ActionReviewPhase::SandboxDenial(_)
        ) {
            "the command requires authority outside the workspace sandbox"
        } else {
            "the action requires user approval"
        };
        Ok(ClassifierAssessment::new(
            AssessmentId::from_response(
                request.action().digest(),
                request.action_policy_revision(),
                LOCAL_REVIEWER_POLICY_REVISION,
                reason,
            ),
            request.action().digest().clone(),
            request.action_policy_revision().clone(),
            LOCAL_REVIEWER_POLICY_REVISION,
            ClassifierRecommendation::AskUser {
                reason: reason.into(),
            },
        ))
    }
}

impl ActionPolicyService for LocalShellPolicy {
    fn revision(&self) -> String {
        self.action_policy_revision.as_str().to_owned()
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        ActionPolicyEngine::new(
            self.action_policy_revision.clone(),
            self.exec_policy.clone(),
            LocalPolicyClassifier,
            ReviewFailurePolicy::Block,
        )
        .decide(request, cancellation)
        .map_err(|error| CoreError::Policy(error.to_string()))
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

    fn policy(error: impl fmt::Display) -> Self {
        Self(format!("could not compose local execution policy: {error}"))
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
