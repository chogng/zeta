use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use zeta_action_policy::ActionDigest;
use zeta_action_policy::ActionKind;
use zeta_action_policy::ActionPolicyRevision;
use zeta_action_policy::ActionProvenance;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ActionSource;
use zeta_action_policy::Capability;
use zeta_action_policy::CapabilityKind;
use zeta_action_policy::CapabilitySet;
use zeta_action_policy::ExecutionDecision;
use zeta_action_policy::ProcessInvocationKind;
use zeta_action_policy::ResolvedAction;
use zeta_action_policy::SandboxCompatibility;
use zeta_async_utils::CancellationToken;
use zeta_config::HookAction;
use zeta_config::HookConfig;
use zeta_config::HookEnablement;
use zeta_config::HookEvent as ConfigHookEvent;
use zeta_config::HooksConfig;
use zeta_core::ActionPolicyService;
use zeta_core::CoreError;
use zeta_core::HookEvent;
use zeta_core::HookService;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::NetworkAccess;
use zeta_sandboxing::SandboxPolicy;
use zeta_tool_executor::ApprovalPolicy;
use zeta_tool_executor::ApprovalRequirement;
use zeta_tool_executor::CommandExecutionAuthority;
use zeta_tool_executor::CommandExecutionOutcome;
use zeta_tool_executor::CommandExecutor;
use zeta_tool_executor::CommandRequest;
use zeta_tool_executor::ExecutionError;
use zeta_tool_executor::ExecutionLimits;
use zeta_workspace::WorkspaceRoot;

const HOOK_TIMEOUT: Duration = Duration::from_secs(30);
const HOOK_OUTPUT_BYTES: usize = 64 * 1024;

/// Host-owned runtime for declarative Hooks.
///
/// The runtime keeps configuration separate from the current trusted Workspace executor. A
/// restricted Workspace therefore has no process runner at all, while a configuration update can
/// replace the immutable Hook snapshot without rebuilding Core's Turn executor.
pub(crate) struct HookRuntime {
    config: RwLock<HooksConfig>,
    policy: Arc<dyn ActionPolicyService>,
    process: RwLock<Option<Arc<dyn HookProcessExecutor>>>,
}

impl HookRuntime {
    pub(crate) fn new(config: HooksConfig, policy: Arc<dyn ActionPolicyService>) -> Self {
        Self {
            config: RwLock::new(config),
            policy,
            process: RwLock::new(None),
        }
    }

    pub(crate) fn replace_config(&self, config: HooksConfig) {
        *self
            .config
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = config;
    }

    /// Installs a process executor only for a trusted, active Workspace.
    pub(crate) fn activate(&self, workspace: WorkspaceRoot) -> Result<(), String> {
        let has_enabled_hooks = self
            .config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .hooks
            .values()
            .any(|hook| hook.enablement == HookEnablement::Enabled);
        if !has_enabled_hooks {
            self.clear_workspace();
            return Ok(());
        }
        let process = NativeHookProcessExecutor::new(workspace)?;
        *self
            .process
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(process));
        Ok(())
    }

    pub(crate) fn clear_workspace(&self) {
        *self
            .process
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    #[cfg(test)]
    fn with_process(
        config: HooksConfig,
        policy: Arc<dyn ActionPolicyService>,
        process: Arc<dyn HookProcessExecutor>,
    ) -> Self {
        Self {
            config: RwLock::new(config),
            policy,
            process: RwLock::new(Some(process)),
        }
    }

    fn run_event(
        &self,
        event: &HookEvent,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let config = self
            .config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let process = self
            .process
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let Some(process) = process else {
            return Ok(());
        };
        for hook in config.hooks.values() {
            cancellation
                .check()
                .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
            if hook.enablement != HookEnablement::Enabled || !matches_event(hook, event) {
                continue;
            }
            let review = review_request(hook, process.workspace(), self.policy.revision())?;
            let decision = self.policy.decide(&review, cancellation)?;
            let authority = match decision {
                ExecutionDecision::RunSandboxed(policy) => {
                    CommandExecutionAuthority::Sandboxed(policy)
                }
                ExecutionDecision::RunExecPolicyGranted(grant) => {
                    if !grant.matches(
                        review.action().digest(),
                        review.action().required_capabilities(),
                        review.action_policy_revision(),
                    ) {
                        return Err(CoreError::Policy(format!(
                            "Hook '{}' received an execution-policy grant for another action",
                            hook.id
                        )));
                    }
                    CommandExecutionAuthority::Unrestricted
                }
                ExecutionDecision::RunAutoReviewed(grant) => {
                    if !grant.matches(
                        review.action().digest(),
                        review.action().required_capabilities(),
                        review.action_policy_revision(),
                    ) {
                        return Err(CoreError::Policy(format!(
                            "Hook '{}' received an automatic-review grant for another action",
                            hook.id
                        )));
                    }
                    CommandExecutionAuthority::Unrestricted
                }
                ExecutionDecision::RunUnsandboxed { .. } => CommandExecutionAuthority::Unrestricted,
                ExecutionDecision::AskUser(_) => {
                    return Err(CoreError::Policy(format!(
                        "Hook '{}' requires interactive approval and was not executed",
                        hook.id
                    )));
                }
                ExecutionDecision::RunWithPermissionBypass(_)
                | ExecutionDecision::ReviseAction(_)
                | ExecutionDecision::Block(_) => {
                    return Err(CoreError::Policy(format!(
                        "Hook '{}' was blocked by policy",
                        hook.id
                    )));
                }
            };
            process.execute(hook, authority, cancellation)?;
        }
        Ok(())
    }
}

impl HookService for HookRuntime {
    fn run(&self, event: &HookEvent, cancellation: &CancellationToken) -> Result<(), CoreError> {
        self.run_event(event, cancellation)
    }
}

trait HookProcessExecutor: Send + Sync {
    fn workspace(&self) -> &WorkspaceRoot;

    fn execute(
        &self,
        hook: &HookConfig,
        authority: CommandExecutionAuthority,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError>;
}

struct AlwaysAuthorized;

impl ApprovalPolicy for AlwaysAuthorized {
    fn requirement_for(&self, _: &str) -> ApprovalRequirement {
        ApprovalRequirement::NotRequired
    }
}

struct NativeHookProcessExecutor {
    workspace: WorkspaceRoot,
    executor: CommandExecutor<AlwaysAuthorized, NativeSandbox>,
}

impl NativeHookProcessExecutor {
    fn new(workspace: WorkspaceRoot) -> Result<Self, String> {
        let backend = native_sandbox().map_err(|error| error.to_string())?;
        Ok(Self {
            workspace: workspace.clone(),
            executor: CommandExecutor::new(
                workspace,
                backend,
                AlwaysAuthorized,
                ExecutionLimits {
                    timeout: HOOK_TIMEOUT,
                    max_output_bytes: HOOK_OUTPUT_BYTES,
                },
            ),
        })
    }
}

impl HookProcessExecutor for NativeHookProcessExecutor {
    fn workspace(&self) -> &WorkspaceRoot {
        &self.workspace
    }

    fn execute(
        &self,
        hook: &HookConfig,
        authority: CommandExecutionAuthority,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError> {
        let HookAction::Process { program, args } = &hook.action;
        let result = self.executor.execute(
            CommandRequest {
                program: program.clone(),
                arguments: args.clone(),
                working_directory: self.workspace.canonical_path().to_path_buf(),
            },
            authority,
            cancellation,
        );
        match result {
            Ok(CommandExecutionOutcome::Completed(output)) if output.exit_code == Some(0) => Ok(()),
            Ok(CommandExecutionOutcome::Completed(_)) => Err(CoreError::Execution(format!(
                "Hook '{}' exited unsuccessfully",
                hook.id
            ))),
            Ok(CommandExecutionOutcome::SandboxDenied(_)) => Err(CoreError::Policy(format!(
                "Hook '{}' was denied by the Workspace sandbox",
                hook.id
            ))),
            Err(error) => Err(CoreError::Execution(hook_execution_error(error))),
        }
    }
}

fn review_request(
    hook: &HookConfig,
    workspace: &WorkspaceRoot,
    policy_revision: String,
) -> Result<ActionReviewRequest, CoreError> {
    let HookAction::Process { program, args } = &hook.action;
    let canonical = serde_json::to_vec(&serde_json::json!({
        "hook_id": hook.id.as_str(),
        "program": program,
        "arguments": args,
        "working_directory": workspace.canonical_path(),
    }))
    .map_err(|error| CoreError::Policy(format!("could not canonicalize Hook action: {error}")))?;
    let capabilities = CapabilitySet::new([
        Capability::new(
            CapabilityKind::FileRead,
            workspace.canonical_path().display().to_string(),
        ),
        Capability::new(
            CapabilityKind::FileWrite,
            workspace.canonical_path().display().to_string(),
        ),
        Capability::new(CapabilityKind::ProcessSpawn, program.clone()),
    ]);
    Ok(ActionReviewRequest::new(
        ResolvedAction::new(
            ActionDigest::from_canonical_bytes(canonical),
            ActionKind::LocalProcess(ProcessInvocationKind::Direct),
            format!("run configured Hook '{}'", hook.id),
            capabilities,
        ),
        ActionProvenance::new(ActionSource::User, hook.id.as_str()),
        SandboxCompatibility::Supported(SandboxPolicy::new(
            FileSystemAccess::WorkspaceWrite,
            NetworkAccess::Denied,
        )),
        ActionPolicyRevision::new(policy_revision),
    ))
}

fn matches_event(hook: &HookConfig, event: &HookEvent) -> bool {
    let (config_event, tool_name) = match event {
        HookEvent::BeforeTool { tool_name } => (ConfigHookEvent::BeforeTool, Some(tool_name)),
        HookEvent::AfterTool { tool_name, .. } => (ConfigHookEvent::AfterTool, Some(tool_name)),
        HookEvent::TurnCompleted => (ConfigHookEvent::TurnCompleted, None),
    };
    if hook.event != config_event {
        return false;
    }
    match tool_name {
        Some(tool_name) => {
            hook.matcher.tool_names.is_empty() || hook.matcher.tool_names.contains(tool_name)
        }
        None => hook.matcher.tool_names.is_empty(),
    }
}

fn hook_execution_error(error: ExecutionError) -> String {
    match error {
        ExecutionError::ApprovalRequired => "Hook execution unexpectedly required approval".into(),
        ExecutionError::Denied => "Hook execution was denied".into(),
        ExecutionError::Spawn(_) => "Hook process could not be started".into(),
        ExecutionError::CancelledBeforeStart(reason) => {
            format!("Hook was cancelled before process start: {reason}")
        }
        ExecutionError::CancelledAfterStart(reason) => {
            format!("Hook was cancelled after process start: {reason}")
        }
        ExecutionError::TimedOut => "Hook process timed out".into(),
        ExecutionError::Sandbox(_) => "Hook sandbox preparation failed".into(),
    }
}

#[cfg(target_os = "macos")]
type NativeSandbox = zeta_sandboxing::MacosSeatbeltSandbox;

#[cfg(target_os = "macos")]
fn native_sandbox() -> Result<NativeSandbox, String> {
    Ok(NativeSandbox::new())
}

#[cfg(target_os = "linux")]
type NativeSandbox = zeta_linux_sandbox::LinuxSandbox;

#[cfg(target_os = "linux")]
fn native_sandbox() -> Result<NativeSandbox, String> {
    NativeSandbox::discover(&zeta_install_context::InstallContext::current())
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
type NativeSandbox = zeta_windows_sandbox::WindowsSandbox;

#[cfg(target_os = "windows")]
fn native_sandbox() -> Result<NativeSandbox, String> {
    NativeSandbox::discover(&zeta_install_context::InstallContext::current())
        .map_err(|error| error.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!("configured Hooks require a supported sandbox backend");

#[cfg(test)]
#[path = "hook_runtime_tests.rs"]
mod tests;
