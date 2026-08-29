use crate::matcher::matches_event;
use crate::outcome::HookDecision;
use crate::policy::execution_authority;
use crate::process::HookProcessExecutor;
use crate::process::NativeHookProcessExecutor;
use crate::protocol::HookInvocation;
use crate::protocol::encode_input;
use crate::records::HookRunLog;
use crate::records::HookRunRecord;
use std::collections::BTreeMap;
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
use zeta_core::HookExecutionEvent;
use zeta_core::HookExecutionObserver;
use zeta_core::HookService;
use zeta_core::NoHookExecutionObserver;
use zeta_core::TurnCompletedHookRequest;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_workspace::TrustedWorkspace;
use zeta_workspace::WorkspaceRoot;

struct SessionHookBinding {
    config: HooksConfig,
    discovery: TrustedWorkspace,
    execution: TrustedWorkspace,
    process: Arc<dyn HookProcessExecutor>,
}

struct ThreadHookBinding {
    workspace: WorkspaceRoot,
    process: Option<Arc<dyn HookProcessExecutor>>,
}

/// Shared host runtime for declarative Hooks.
///
/// The runtime keeps configuration separate from the current trusted Workspace executor. A
/// restricted Workspace therefore has no process runner at all, while a configuration update can
/// replace the immutable Hook snapshot without rebuilding Core's Turn executor.
pub struct DeclarativeHookRuntime {
    config: RwLock<HooksConfig>,
    policy: Arc<dyn ActionPolicyService>,
    process: RwLock<Option<Arc<dyn HookProcessExecutor>>>,
    thread_bindings: RwLock<BTreeMap<ThreadId, ThreadHookBinding>>,
    session_bindings: RwLock<BTreeMap<SessionId, Vec<SessionHookBinding>>>,
    execution_observer: RwLock<Arc<dyn HookExecutionObserver>>,
    runs: HookRunLog,
}

impl DeclarativeHookRuntime {
    /// Creates an unbound runtime from an initial declaration snapshot and host policy.
    pub fn new(config: HooksConfig, policy: Arc<dyn ActionPolicyService>) -> Self {
        Self {
            config: RwLock::new(config),
            policy,
            process: RwLock::new(None),
            thread_bindings: RwLock::new(BTreeMap::new()),
            session_bindings: RwLock::new(BTreeMap::new()),
            execution_observer: RwLock::new(Arc::new(NoHookExecutionObserver)),
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

    /// Binds primary Hook execution for one Thread to its managed workspace.
    pub fn bind_thread_workspace(
        &self,
        thread_id: ThreadId,
        workspace: WorkspaceRoot,
    ) -> Result<(), HookWorkspaceBindingError> {
        let process = if has_enabled_hooks(
            &self
                .config
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            Some(Arc::new(
                NativeHookProcessExecutor::new(workspace.clone())
                    .map_err(|message| HookWorkspaceBindingError { message })?,
            ) as Arc<dyn HookProcessExecutor>)
        } else {
            None
        };
        self.thread_bindings
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(thread_id, ThreadHookBinding { workspace, process });
        Ok(())
    }

    pub fn unbind_thread_workspace(&self, thread_id: &ThreadId) {
        self.thread_bindings
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(thread_id);
    }

    pub fn set_execution_observer(&self, observer: Arc<dyn HookExecutionObserver>) {
        *self
            .execution_observer
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = observer;
    }

    /// Replaces additional-directory Hook bindings for one Session.
    pub fn replace_session_workspaces(
        &self,
        session_id: SessionId,
        workspaces: Vec<(HooksConfig, TrustedWorkspace, TrustedWorkspace)>,
    ) -> Result<(), HookWorkspaceBindingError> {
        let mut bindings = Vec::new();
        for (config, discovery, execution) in workspaces {
            discovery
                .ensure_active()
                .map_err(|error| HookWorkspaceBindingError {
                    message: error.to_string(),
                })?;
            execution
                .ensure_active()
                .map_err(|error| HookWorkspaceBindingError {
                    message: error.to_string(),
                })?;
            let process = Arc::new(
                NativeHookProcessExecutor::new(execution.root().clone())
                    .map_err(|message| HookWorkspaceBindingError { message })?,
            );
            bindings.push(SessionHookBinding {
                config,
                discovery,
                execution,
                process,
            });
        }
        let mut sessions = self
            .session_bindings
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if bindings.is_empty() {
            sessions.remove(&session_id);
        } else {
            sessions.insert(session_id, bindings);
        }
        Ok(())
    }

    pub fn remove_session(&self, session_id: &SessionId) {
        self.session_bindings
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id);
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
            thread_bindings: RwLock::new(BTreeMap::new()),
            session_bindings: RwLock::new(BTreeMap::new()),
            execution_observer: RwLock::new(Arc::new(NoHookExecutionObserver)),
            runs: HookRunLog::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn bind_thread_process(
        &self,
        thread_id: ThreadId,
        process: Arc<dyn HookProcessExecutor>,
    ) {
        self.thread_bindings
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                thread_id,
                ThreadHookBinding {
                    workspace: process.workspace().clone(),
                    process: Some(process),
                },
            );
    }

    fn run_event(
        &self,
        session_id: &SessionId,
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
            .thread_process(invocation.thread_id(), &config)?
            .or_else(|| {
                self.process
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone()
            });
        if let Some(process) = process {
            let decision = self.run_config(&config, process.as_ref(), invocation, cancellation)?;
            if matches!(decision, HookDecision::Deny { .. }) {
                return Ok(decision);
            }
        }
        let sessions = self
            .session_bindings
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(bindings) = sessions.get(session_id) {
            for binding in bindings {
                if binding.discovery.ensure_active().is_err()
                    || binding.execution.ensure_active().is_err()
                {
                    continue;
                }
                let decision = self.run_config(
                    &binding.config,
                    binding.process.as_ref(),
                    invocation,
                    cancellation,
                )?;
                if matches!(decision, HookDecision::Deny { .. }) {
                    return Ok(decision);
                }
            }
        }
        Ok(HookDecision::Continue)
    }

    fn run_config(
        &self,
        config: &HooksConfig,
        process: &dyn HookProcessExecutor,
        invocation: &HookInvocation<'_>,
        cancellation: &CancellationToken,
    ) -> Result<HookDecision, CoreError> {
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
                let event = HookExecutionEvent {
                    session_id: invocation.session_id().clone(),
                    thread_id: invocation.thread_id().clone(),
                    turn_id: invocation.turn_id().clone(),
                    hook_id: hook.id.to_string(),
                    workspace: process.workspace().canonical_path().to_path_buf(),
                };
                let observer = self
                    .execution_observer
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                observer.will_execute(&event)?;
                let result = process.execute(hook, input, authority, cancellation);
                observer.did_finish(&event);
                result
            })();
            self.runs.finish(started, &result);
            let decision = result?;
            if matches!(decision, HookDecision::Deny { .. }) {
                return Ok(decision);
            }
        }
        Ok(HookDecision::Continue)
    }

    fn thread_process(
        &self,
        thread_id: &ThreadId,
        config: &HooksConfig,
    ) -> Result<Option<Arc<dyn HookProcessExecutor>>, CoreError> {
        let mut bindings = self
            .thread_bindings
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(binding) = bindings.get_mut(thread_id) else {
            return Ok(None);
        };
        if binding.process.is_none() && has_enabled_hooks(config) {
            binding.process = Some(Arc::new(
                NativeHookProcessExecutor::new(binding.workspace.clone())
                    .map_err(CoreError::Execution)?,
            ));
        }
        Ok(binding.process.clone())
    }
}

fn has_enabled_hooks(config: &HooksConfig) -> bool {
    config
        .hooks
        .values()
        .any(|hook| hook.enablement == HookEnablement::Enabled)
}

impl HookService for DeclarativeHookRuntime {
    fn before_tool(
        &self,
        request: &BeforeToolHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<BeforeToolHookDecision, CoreError> {
        match self.run_event(
            &request.session_id,
            &HookInvocation::BeforeTool(request),
            cancellation,
        )? {
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
            self.run_event(
                &request.session_id,
                &HookInvocation::AfterTool(request),
                cancellation,
            )?,
            "afterTool",
        )
    }

    fn turn_completed(
        &self,
        request: &TurnCompletedHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError> {
        require_observational_result(
            self.run_event(
                &request.session_id,
                &HookInvocation::TurnCompleted(request),
                cancellation,
            )?,
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
