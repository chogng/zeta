use crate::matcher::matches_event;
use crate::outcome::HookDecision;
use crate::policy::execution_authority;
use crate::process::HookProcessExecutor;
use crate::process::NativeHookProcessExecutor;
use crate::protocol::HookInvocation;
use crate::protocol::encode_input;
use crate::records::HookRunLog;
use crate::records::HookRunRecord;
use std::sync::Arc;
use std::sync::RwLock;
use zeta_async_utils::CancellationToken;
use zeta_config::HookEnablement;
use zeta_config::HooksConfig;
use zeta_core::ActionPolicyService;
use zeta_core::AfterToolHookRequest;
use zeta_core::BeforeToolHookDecision;
use zeta_core::BeforeToolHookRequest;
use zeta_core::CoreError;
use zeta_core::HookService;
use zeta_core::TurnCompletedHookRequest;
use zeta_workspace::WorkspaceRoot;

/// Shared host runtime for declarative Hooks.
///
/// The runtime keeps configuration separate from the current trusted Workspace executor. A
/// restricted Workspace therefore has no process runner at all, while a configuration update can
/// replace the immutable Hook snapshot without rebuilding Core's Turn executor.
pub struct DeclarativeHookRuntime {
    config: RwLock<HooksConfig>,
    policy: Arc<dyn ActionPolicyService>,
    process: RwLock<Option<Arc<dyn HookProcessExecutor>>>,
    runs: HookRunLog,
}

impl DeclarativeHookRuntime {
    /// Creates an unbound runtime from an initial declaration snapshot and host policy.
    pub fn new(config: HooksConfig, policy: Arc<dyn ActionPolicyService>) -> Self {
        Self {
            config: RwLock::new(config),
            policy,
            process: RwLock::new(None),
            runs: HookRunLog::new(),
        }
    }

    /// Replaces the declaration snapshot used by future Hook invocations.
    pub fn replace_config(&self, config: HooksConfig) {
        *self
            .config
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = config;
    }

    /// Binds process execution to an active Workspace that the host has already trusted.
    pub fn bind_workspace(
        &self,
        workspace: WorkspaceRoot,
    ) -> Result<(), HookWorkspaceBindingError> {
        let has_enabled_hooks = self
            .config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .hooks
            .values()
            .any(|hook| hook.enablement == HookEnablement::Enabled);
        if !has_enabled_hooks {
            self.unbind_workspace();
            return Ok(());
        }
        let process = NativeHookProcessExecutor::new(workspace)
            .map_err(|message| HookWorkspaceBindingError { message })?;
        *self
            .process
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(process));
        Ok(())
    }

    /// Removes the active Workspace executor so future Hook invocations cannot spawn processes.
    pub fn unbind_workspace(&self) {
        *self
            .process
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    /// Returns the bounded, non-durable projection of recent Hook invocations.
    pub fn recent_runs(&self) -> Vec<HookRunRecord> {
        self.runs.snapshot()
    }

    #[cfg(test)]
    pub(crate) fn with_process(
        config: HooksConfig,
        policy: Arc<dyn ActionPolicyService>,
        process: Arc<dyn HookProcessExecutor>,
    ) -> Self {
        Self {
            config: RwLock::new(config),
            policy,
            process: RwLock::new(Some(process)),
            runs: HookRunLog::new(),
        }
    }

    fn run_event(
        &self,
        invocation: &HookInvocation<'_>,
        cancellation: &CancellationToken,
    ) -> Result<HookDecision, CoreError> {
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
            return Ok(HookDecision::Continue);
        };
        for hook in config.hooks.values() {
            cancellation
                .check()
                .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
            if hook.enablement != HookEnablement::Enabled || !matches_event(hook, invocation) {
                continue;
            }
            let started = self.runs.start(hook, invocation);
            let result = (|| {
                let authority = execution_authority(
                    hook,
                    process.workspace(),
                    self.policy.as_ref(),
                    cancellation,
                )?;
                let input = encode_input(hook, invocation, process.workspace().canonical_path())?;
                process.execute(hook, input, authority, cancellation)
            })();
            self.runs.finish(started, &result);
            let decision = result?;
            if matches!(decision, HookDecision::Deny { .. }) {
                return Ok(decision);
            }
        }
        Ok(HookDecision::Continue)
    }
}

impl HookService for DeclarativeHookRuntime {
    fn before_tool(
        &self,
        request: &BeforeToolHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<BeforeToolHookDecision, CoreError> {
        match self.run_event(&HookInvocation::BeforeTool(request), cancellation)? {
            HookDecision::Continue => Ok(BeforeToolHookDecision::Continue),
            HookDecision::Deny { reason } => Ok(BeforeToolHookDecision::Deny { reason }),
        }
    }

    fn after_tool(
        &self,
        request: &AfterToolHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError> {
        require_observational_result(
            self.run_event(&HookInvocation::AfterTool(request), cancellation)?,
            "afterTool",
        )
    }

    fn turn_completed(
        &self,
        request: &TurnCompletedHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError> {
        require_observational_result(
            self.run_event(&HookInvocation::TurnCompleted(request), cancellation)?,
            "turnCompleted",
        )
    }
}

fn require_observational_result(decision: HookDecision, event_name: &str) -> Result<(), CoreError> {
    match decision {
        HookDecision::Continue => Ok(()),
        HookDecision::Deny { .. } => Err(CoreError::Execution(format!(
            "{event_name} Hook cannot deny an operation that has already completed"
        ))),
    }
}

/// Failure to construct the sandboxed process executor for a trusted Workspace.
#[derive(Debug)]
pub struct HookWorkspaceBindingError {
    message: String,
}

impl std::fmt::Display for HookWorkspaceBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "could not bind declarative Hooks to the Workspace: {}",
            self.message
        )
    }
}

impl std::error::Error for HookWorkspaceBindingError {}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
