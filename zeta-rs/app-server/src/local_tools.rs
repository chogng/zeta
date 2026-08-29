use serde_json::json;
use std::fmt;
use std::path::Component;
use std::path::Path;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use zeta_action_policy::ActionClassifier;
use zeta_action_policy::ActionDigest;
use zeta_action_policy::ActionKind;
use zeta_action_policy::ActionPolicyEngine;
use zeta_action_policy::ActionPolicyRevision;
use zeta_action_policy::ActionProvenance;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ActionSource;
use zeta_action_policy::AssessmentId;
use zeta_action_policy::Capability;
use zeta_action_policy::CapabilityKind;
use zeta_action_policy::CapabilitySet;
use zeta_action_policy::ClassifierAssessment;
use zeta_action_policy::ClassifierRecommendation;
use zeta_action_policy::ExecutionDecision;
use zeta_action_policy::ProcessInvocationKind;
use zeta_action_policy::ResolvedAction;
use zeta_action_policy::ReviewFailurePolicy;
use zeta_action_policy::SandboxCompatibility;
use zeta_apply_patch::ApplyPatchLimits;
use zeta_apply_patch::ApplyPatchTool;
use zeta_async_utils::CancellationToken;
use zeta_config::AgentGrepBackend;
use zeta_config::ResolvedConfig;
use zeta_config::UserExecPolicyConfig;
use zeta_config::WorkspaceExecPolicyConfig;
use zeta_config::WorkspaceId;
use zeta_core::ActionPolicyService;
use zeta_core::CoreError;
use zeta_core::ToolAuthorization;
use zeta_core::ToolExecutionFacts;
use zeta_core::ToolOutputSink;
use zeta_core::ToolService;
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
use zeta_install_context::ExecutableCandidates;
use zeta_install_context::InstallContext;
use zeta_install_context::ManagedExecutable;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolOutputStream;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::NetworkAccess;
use zeta_sandboxing::SandboxBackend;
use zeta_sandboxing::SandboxPolicy;
use zeta_shell_command::ApprovalPolicy;
use zeta_shell_command::ApprovalRequirement;
use zeta_shell_command::CommandExecutionAuthority;
use zeta_shell_command::CommandExecutionOutcome;
use zeta_shell_command::ExecutionError;
use zeta_shell_command::RipgrepDiscoveryError;
use zeta_shell_command::RipgrepExecutable;
use zeta_shell_command::ShellCommandLimits;
use zeta_shell_command::ShellCommandRequest;
use zeta_shell_command::ShellCommandTool;
use zeta_tools::ToolPayload;
use zeta_tools::to_protocol_tool_definition;
use zeta_workspace::TrustedWorkspace;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceRoot;

use crate::session_workspace_access::SessionWorkspaceAccess;
use crate::tool_composition::ToolCompositionError;
use crate::tool_composition::ToolPort;
use crate::tool_executor_adapter::PreparedToolExecution;
use crate::tool_executor_adapter::ToolExecutorReviewer;

mod agent_grep;
mod suite;

pub(crate) use agent_grep::AgentGrepService;
pub(crate) use suite::LocalToolSuite;

const LOCAL_GRANT_SNAPSHOT_REVISION: &str = "local-static-grants-v1";
const LOCAL_REVIEWER_POLICY_REVISION: &str = "local-interactive-review-v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_OUTPUT_BYTES: usize = 256 * 1024;

pub(crate) struct LocalToolComposition {
    pub(crate) tools: Arc<dyn ToolService>,
    pub(crate) policy: Arc<dyn ActionPolicyService>,
    pub(crate) ripgrep: RipgrepExecutable,
    pub(crate) agent_grep: Arc<AgentGrepService>,
    action_policy_revision: ActionPolicyRevision,
    executors: Vec<LocalExecutorContribution>,
}

/// Durable configuration inputs used to compose the local Agent Tool suite.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LocalToolConfig {
    user: UserExecPolicyConfig,
    workspace: Option<(WorkspaceId, WorkspaceExecPolicyConfig)>,
    agent_grep_backend: AgentGrepBackend,
}

impl LocalToolConfig {
    pub(crate) fn from_resolved(config: &ResolvedConfig) -> Self {
        Self {
            user: config.exec_policy.clone(),
            workspace: config.workspace.as_ref().map(|workspace| {
                (
                    workspace.workspace_id.clone(),
                    workspace.exec_policy.clone(),
                )
            }),
            agent_grep_backend: config.agent_grep_backend,
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
    config: &LocalToolConfig,
    session_workspace_access: Arc<SessionWorkspaceAccess>,
    existing_agent_grep: Option<Arc<AgentGrepService>>,
    workspace_index_storage: Option<Arc<zeta_workspace_index_storage::WorkspaceIndexStorage>>,
    fast_regex_worker_command: Option<&zeta_fast_regex_search::FastRegexWorkerCommand>,
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
    let exec_policy = config.snapshot()?;
    let action_policy_revision = ActionPolicyRevision::from_components(
        exec_policy.revision(),
        LOCAL_GRANT_SNAPSHOT_REVISION,
        LOCAL_REVIEWER_POLICY_REVISION,
    );
    let reviewer: Arc<dyn ToolExecutorReviewer> = Arc::new(LocalExecutorReviewer {
        workspace: workspace.clone(),
        ripgrep: ripgrep.clone(),
        action_policy_revision: action_policy_revision.clone(),
        session_workspace_access: Arc::clone(&session_workspace_access),
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
    let agent_grep = Arc::new(match existing_agent_grep {
        Some(existing) => existing.reconfigured(config.agent_grep_backend, ripgrep.clone()),
        None => match (&workspace_index_storage, fast_regex_worker_command) {
            (Some(storage), Some(worker_command)) => AgentGrepService::new_with_worker(
                config.agent_grep_backend,
                ripgrep.clone(),
                Arc::clone(storage),
                worker_command.clone(),
            ),
            _ => AgentGrepService::new(
                config.agent_grep_backend,
                ripgrep.clone(),
                workspace_index_storage,
            ),
        },
    });
    let service = LocalToolSuite::new(
        shell,
        ripgrep.clone(),
        Arc::clone(&agent_grep),
        session_workspace_access,
    );
    Ok(LocalToolComposition {
        tools: Arc::new(service),
        policy: Arc::new(policy),
        ripgrep,
        agent_grep,
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
        let agent_grep = Arc::new(AgentGrepService::new(
            AgentGrepBackend::Ripgrep,
            ripgrep.clone(),
            None,
        ));
        Self {
            tools,
            policy,
            ripgrep,
            agent_grep,
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
        agent_grep: composition.agent_grep,
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
    session_workspace_access: Arc<SessionWorkspaceAccess>,
}

impl ToolExecutorReviewer for LocalExecutorReviewer {
    fn prepare(&self, call: &ToolCall) -> Result<PreparedToolExecution, CoreError> {
        self.prepare_for_session(call, None)
    }

    fn prepare_with_facts(
        &self,
        call: &ToolCall,
        facts: &ToolExecutionFacts,
    ) -> Result<PreparedToolExecution, CoreError> {
        let session_id = facts
            .execution_identity()
            .map(|identity| identity.session_id())
            .ok_or_else(|| {
                CoreError::Policy("local executors require durable caller identity".into())
            })?;
        self.prepare_for_session(call, Some(session_id))
    }
}

impl LocalExecutorReviewer {
    fn prepare_for_session(
        &self,
        call: &ToolCall,
        session_id: Option<&zeta_protocol::SessionId>,
    ) -> Result<PreparedToolExecution, CoreError> {
        self.workspace
            .ensure_active()
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        if call.name.as_str() == "shell-command" {
            let (review, request, workspace) = self.prepare_shell(call, session_id)?;
            return Ok(PreparedToolExecution::new(
                review,
                ToolPayload::FunctionArguments(json!({
                    "program": request.program(),
                    "arguments": request.arguments(),
                    "working_directory": request.working_directory(),
                    "workspace_root": request.workspace_root(),
                })),
            )
            .with_workspace_guard(workspace));
        }
        if call.name.as_str() == "apply_patch" {
            let (review, patch, workspace) = self.prepare_apply_patch(call, session_id)?;
            return Ok(PreparedToolExecution::new(
                review,
                ToolPayload::FunctionArguments(json!({
                    "patch": patch,
                    "workspace_root": workspace.root().canonical_path(),
                })),
            )
            .with_workspace_guard(workspace));
        }
        let review = match call.name.as_str() {
            _ => Err(CoreError::Policy(format!(
                "local executor reviewer does not own tool {}",
                call.name
            ))),
        }?;
        Ok(PreparedToolExecution::new(
            review,
            ToolPayload::FunctionArguments(call.arguments.clone()),
        )
        .with_workspace_guard(self.workspace.clone()))
    }

    fn prepare_shell(
        &self,
        call: &ToolCall,
        session_id: Option<&zeta_protocol::SessionId>,
    ) -> Result<(ActionReviewRequest, ShellCommandRequest, TrustedWorkspace), CoreError> {
        if call.arguments.get("workspace_root").is_some() {
            return Err(CoreError::Policy(
                "shell-command workspace_root is host-owned".into(),
            ));
        }
        let mut request = ShellCommandRequest::from_arguments(&ToolPayload::FunctionArguments(
            call.arguments.clone(),
        ))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        let (workspace, relative_working_directory, working_directory) =
            self.resolve_execution_workspace(request.working_directory(), session_id)?;
        request = request
            .with_working_directory(relative_working_directory)
            .with_workspace_root(workspace.root().canonical_path());
        if request.program() == "rg" {
            validate_workspace_arguments(workspace.root(), &request)?;
        }
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
            local_capabilities(workspace.root(), &self.ripgrep)
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
        Ok((review, request, workspace))
    }

    fn resolve_execution_workspace(
        &self,
        requested: &Path,
        session_id: Option<&zeta_protocol::SessionId>,
    ) -> Result<(TrustedWorkspace, std::path::PathBuf, std::path::PathBuf), CoreError> {
        if requested.is_relative() {
            let absolute = self
                .workspace
                .root()
                .resolve_existing(requested)
                .map_err(|error| CoreError::Policy(error.to_string()))?;
            return Ok((self.workspace.clone(), requested.to_path_buf(), absolute));
        }
        let mut workspaces = vec![self.workspace.clone()];
        if let Some(session_id) = session_id
            && let Some(snapshot) = self
                .session_workspace_access
                .snapshot_for(session_id, WorkspaceCapability::ExecuteProcess)
                .map_err(|error| CoreError::Policy(error.to_string()))?
        {
            workspaces.extend(snapshot.additional_roots().iter().cloned());
        }
        let (workspace, relative) = workspaces
            .into_iter()
            .filter_map(|workspace| {
                requested
                    .strip_prefix(workspace.root().canonical_path())
                    .or_else(|_| requested.strip_prefix(workspace.root().requested_path()))
                    .ok()
                    .map(|relative| (workspace, relative.to_path_buf()))
            })
            .max_by_key(|(workspace, _)| workspace.root().canonical_path().components().count())
            .ok_or_else(|| {
                CoreError::Policy(format!(
                    "shell-command working directory is not authorized: {}",
                    requested.display()
                ))
            })?;
        let relative = if relative.as_os_str().is_empty() {
            std::path::PathBuf::from(".")
        } else {
            relative
        };
        let absolute = workspace
            .root()
            .resolve_existing(&relative)
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        Ok((workspace, relative, absolute))
    }

    fn prepare_apply_patch(
        &self,
        call: &ToolCall,
        session_id: Option<&zeta_protocol::SessionId>,
    ) -> Result<(ActionReviewRequest, String, TrustedWorkspace), CoreError> {
        if call.arguments.get("workspace_root").is_some() {
            return Err(CoreError::Policy(
                "apply_patch workspace_root is host-owned".into(),
            ));
        }
        let patch = call
            .arguments
            .get("patch")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| CoreError::Policy("apply_patch patch must be a string".into()))?;
        let mut workspaces = vec![self.workspace.clone()];
        if let Some(session_id) = session_id
            && let Some(snapshot) = self
                .session_workspace_access
                .snapshot_for(session_id, WorkspaceCapability::MutateRepository)
                .map_err(|error| CoreError::Policy(error.to_string()))?
        {
            workspaces.extend(snapshot.additional_roots().iter().cloned());
        }
        let (workspace, rewritten_patch, targets) = materialize_patch_targets(&workspaces, patch)?;
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
        let review = ActionReviewRequest::new(
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
        );
        Ok((review, rewritten_patch, workspace))
    }
}

fn materialize_patch_targets(
    workspaces: &[TrustedWorkspace],
    patch: &str,
) -> Result<(TrustedWorkspace, String, Vec<String>), CoreError> {
    let primary = workspaces
        .first()
        .ok_or_else(|| CoreError::Policy("apply_patch has no authorized workspace".into()))?;
    let mut selected: Option<TrustedWorkspace> = None;
    let mut targets = Vec::new();
    let mut rewritten = Vec::new();
    for line in patch.lines() {
        let operation = [
            ("*** Update File: ", true),
            ("*** Delete File: ", true),
            ("*** Add File: ", false),
        ]
        .into_iter()
        .find_map(|(prefix, existing)| line.strip_prefix(prefix).map(|path| (path, existing)));
        let Some((path, existing)) = operation else {
            rewritten.push(line.to_owned());
            continue;
        };
        if path.trim().is_empty() {
            return Err(CoreError::Policy(
                "apply_patch contains an empty target path".into(),
            ));
        }
        let path = Path::new(path);
        let (workspace, relative) = if path.is_absolute() {
            workspaces
                .iter()
                .filter_map(|workspace| {
                    path.strip_prefix(workspace.root().canonical_path())
                        .or_else(|_| path.strip_prefix(workspace.root().requested_path()))
                        .ok()
                        .map(|relative| (workspace.clone(), relative.to_path_buf()))
                })
                .max_by_key(|(workspace, _)| workspace.root().canonical_path().components().count())
                .ok_or_else(|| {
                    CoreError::Policy(format!(
                        "apply_patch target is not in an authorized writable directory: {}",
                        path.display()
                    ))
                })?
        } else {
            (primary.clone(), path.to_path_buf())
        };
        if let Some(selected) = &selected
            && selected.root() != workspace.root()
        {
            return Err(CoreError::Policy(
                "one apply_patch call cannot modify more than one workspace root".into(),
            ));
        }
        selected = Some(workspace.clone());
        let resolved = if existing {
            workspace.root().resolve_existing(&relative)
        } else {
            workspace.root().resolve_for_write(&relative)
        }
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        targets.push(resolved.display().to_string());
        let prefix = if existing && line.starts_with("*** Update File: ") {
            "*** Update File: "
        } else if existing {
            "*** Delete File: "
        } else {
            "*** Add File: "
        };
        rewritten.push(format!("{prefix}{}", relative.display()));
    }
    targets.sort();
    targets.dedup();
    if targets.is_empty() {
        return Err(CoreError::Policy(
            "apply_patch contains no file operations to review".into(),
        ));
    }
    Ok((
        selected.expect("a patch with targets selected one workspace"),
        rewritten.join("\n"),
        targets,
    ))
}

struct LocalShellPolicy {
    exec_policy: ExecPolicySnapshot,
    action_policy_revision: ActionPolicyRevision,
}

impl Default for LocalShellPolicy {
    fn default() -> Self {
        let exec_policy = LocalToolConfig::default()
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
    let exec_policy = LocalToolConfig::default()
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
