use super::workspace_runtime::WorkspaceRuntime;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::Weak;
use zeta_core::CoreError;
use zeta_core::ThreadController;
use zeta_core::TurnExecutionBackend;
use zeta_model_provider_config::find_static_model;
use zeta_protocol::CommandId;
use zeta_protocol::ModelAccess;
use zeta_protocol::ModelRef;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;
use zeta_protocol::UserInput;
use zeta_workspace::WorkspaceCapability;

/// Stable backend handle shared by product Turn dispatch and multi-agent tools.
///
/// Composition replaces the target after all builder-only Workspace mutations finish, while
/// existing consumers retain this handle and therefore observe the canonical router.
pub(crate) struct TurnBackendHandle {
    target: RwLock<Arc<dyn TurnExecutionBackend>>,
}

impl TurnBackendHandle {
    pub(crate) fn new(target: Arc<dyn TurnExecutionBackend>) -> Self {
        Self {
            target: RwLock::new(target),
        }
    }

    pub(crate) fn replace(&self, target: Arc<dyn TurnExecutionBackend>) {
        *self
            .target
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = target;
    }

    fn current(&self) -> Arc<dyn TurnExecutionBackend> {
        Arc::clone(
            &self
                .target
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

impl TurnExecutionBackend for TurnBackendHandle {
    fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.current().start(thread_id, turn_id)
    }

    fn resume(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.current().resume(thread_id, turn_id)
    }

    fn steer(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        command_id: &CommandId,
        input: &[UserInput],
    ) -> Result<(), CoreError> {
        self.current().steer(thread_id, turn_id, command_id, input)
    }
}

/// Selects the complete-Turn backend from the model already persisted on a Core Turn.
///
/// API-key models use the local `TurnExecutor`; static rows whose access mode is subscription use
/// the injected Codex backend. Selection never consults login state, mutable UI state, or
/// model-name heuristics.
pub(crate) struct TurnBackendRouter {
    threads: Arc<ThreadController>,
    local: Arc<dyn TurnExecutionBackend>,
    codex: Arc<dyn TurnExecutionBackend>,
}

impl TurnBackendRouter {
    pub(crate) fn new(
        threads: Arc<ThreadController>,
        local: Arc<dyn TurnExecutionBackend>,
        codex: Arc<dyn TurnExecutionBackend>,
    ) -> Self {
        Self {
            threads,
            local,
            codex,
        }
    }

    fn backend(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
    ) -> Result<Arc<dyn TurnExecutionBackend>, CoreError> {
        let snapshot = self.threads.read_thread(thread_id)?;
        let model = snapshot
            .turns
            .iter()
            .find(|turn| &turn.turn_id == turn_id)
            .ok_or_else(|| CoreError::NotFound(turn_id.to_string()))?
            .model
            .as_ref();
        if model.is_some_and(|model: &ModelRef| {
            find_static_model(model).is_some_and(|entry| entry.access == ModelAccess::Subscription)
        }) {
            Ok(Arc::clone(&self.codex))
        } else {
            Ok(Arc::clone(&self.local))
        }
    }
}

impl TurnExecutionBackend for TurnBackendRouter {
    fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.backend(thread_id, turn_id)?.start(thread_id, turn_id)
    }

    fn resume(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.backend(thread_id, turn_id)?.resume(thread_id, turn_id)
    }

    fn steer(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        command_id: &CommandId,
        input: &[UserInput],
    ) -> Result<(), CoreError> {
        self.backend(thread_id, turn_id)?
            .steer(thread_id, turn_id, command_id, input)
    }
}

/// Delegates to the latest local executor installed for the active workspace.
pub(crate) struct CurrentLocalTurnBackend {
    runtime: Weak<RwLock<WorkspaceRuntime>>,
}

impl CurrentLocalTurnBackend {
    pub(crate) fn new(runtime: &Arc<RwLock<WorkspaceRuntime>>) -> Self {
        Self {
            runtime: Arc::downgrade(runtime),
        }
    }

    fn executor(&self) -> Result<zeta_core::TurnExecutor, CoreError> {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or_else(|| CoreError::Execution("local Turn runtime is unavailable".into()))?;
        let executor = runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .turn_executor
            .clone();
        Ok(executor)
    }
}

impl TurnExecutionBackend for CurrentLocalTurnBackend {
    fn start(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.executor()?.start(thread_id, turn_id)
    }

    fn resume(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), CoreError> {
        self.executor()?.resume(thread_id, turn_id)
    }

    fn steer(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        command_id: &CommandId,
        input: &[UserInput],
    ) -> Result<(), CoreError> {
        self.executor()?
            .steer(thread_id, turn_id, command_id, input)
    }
}

/// Projects the currently authorized App Server workspace into the Codex adapter boundary.
pub(crate) struct CurrentCodexWorkspace {
    runtime: Weak<RwLock<WorkspaceRuntime>>,
}

impl CurrentCodexWorkspace {
    pub(crate) fn new(runtime: &Arc<RwLock<WorkspaceRuntime>>) -> Self {
        Self {
            runtime: Arc::downgrade(runtime),
        }
    }
}

impl zeta_codex_app_server::CodexTurnWorkspaceSource for CurrentCodexWorkspace {
    fn current_workspace(&self) -> Result<zeta_codex_app_server::CodexTurnWorkspace, CoreError> {
        let runtime = self.runtime.upgrade().ok_or_else(|| {
            CoreError::Execution("Codex Turn Workspace runtime is unavailable".into())
        })?;
        let runtime = runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let authorization = runtime.authorization.as_ref().ok_or_else(|| {
            CoreError::Execution("Codex Turn requires an active Workspace".into())
        })?;
        authorization
            .require(WorkspaceCapability::ExecuteProcess)
            .map_err(|_| CoreError::Policy("Codex Turn requires a trusted Workspace".into()))?;
        Ok(zeta_codex_app_server::CodexTurnWorkspace {
            path: authorization.root().canonical_path().to_path_buf(),
            execution_scope: authorization.root().trust_id().to_string(),
        })
    }
}
