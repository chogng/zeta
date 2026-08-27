//! App Server runtime for the Session capability.
//!
//! This module owns the Session/Thread requests, subscription, command queue, and reconnect
//! lifecycle. Other App Server capabilities use the connected request handle emitted to the
//! product host.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::SyncSender;
use std::thread;
use std::thread::JoinHandle;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use zeta_protocol::ModelRef;
use zeta_protocol::SessionId;

pub use crate::runtime_contract::CommandResult;
pub use crate::runtime_contract::SESSION_UNAVAILABLE_COMMAND_ERROR;
use crate::runtime_contract::SessionRuntimeCommand;
pub use crate::runtime_contract::SessionRuntimeEvent;
pub use crate::runtime_contract::WorkspaceSwitchResult;
use crate::runtime_contract::command_channel;

/// Connection authority supplied by the product host.
///
/// Implementations preserve their Local or Remote transport identity when retargeted to another
/// Workspace. The runtime uses this contract without importing product composition types.
pub trait SessionRuntimeTarget: Send + Sync {
    /// Returns whether transport loss should use the bounded reconnect policy.
    fn is_remote(&self) -> bool;

    /// Returns the Workspace root represented by this target.
    fn workspace_root(&self) -> &Path;

    /// Creates the same connection authority retargeted to another Workspace.
    fn retarget(&self, root: &Path) -> CommandResult<Box<dyn SessionRuntimeTarget>>;

    /// Opens and initializes one App Server protocol session.
    fn start(&self) -> CommandResult<zeta_app_server_client::AppServerSession>;
}

pub(crate) type SessionRuntimeEventSink =
    Arc<dyn Fn(SessionRuntimeEvent) -> CommandResult<()> + Send + Sync>;
/// Running Session runtime worker handle used by the product host.
pub struct SessionRuntime {
    available: Arc<AtomicBool>,
    commands: SyncSender<SessionRuntimeCommand>,
    worker: Option<JoinHandle<()>>,
}

impl SessionRuntime {
    /// Starts one worker for the supplied App Server target and product event sink.
    pub fn spawn<T, F>(target: T, event_sink: F) -> Result<Self>
    where
        T: SessionRuntimeTarget + 'static,
        F: Fn(SessionRuntimeEvent) -> CommandResult<()> + Send + Sync + 'static,
    {
        let (commands, command_receiver) = command_channel();
        let available = Arc::new(AtomicBool::new(false));
        let worker_availability = Arc::clone(&available);
        let event_sink: SessionRuntimeEventSink = Arc::new(event_sink);
        let worker = thread::Builder::new()
            .name("app-session-runtime".into())
            .spawn(move || {
                crate::runtime_worker::run_session_runtime(
                    event_sink,
                    command_receiver,
                    Box::new(target),
                    worker_availability,
                )
            })
            .context("could not start Session runtime worker")?;
        Ok(Self {
            available,
            commands,
            worker: Some(worker),
        })
    }

    /// Returns whether the worker currently accepts commands.
    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Acquire)
    }

    /// Submits one text turn to the active Thread.
    pub fn submit_agent_message(&self, text: String) -> Result<()> {
        self.try_send(
            SessionRuntimeCommand::SubmitAgentMessage(text),
            "Session submission queue is unavailable",
        )
    }

    /// Creates and activates a Session in the current Workspace.
    pub fn create_session(&self) -> Result<()> {
        self.try_send(
            SessionRuntimeCommand::CreateSession,
            "Session creation queue is unavailable",
        )
    }

    /// Stops an active Session.
    pub fn stop_session(&self, session_id: SessionId) -> Result<()> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            SessionRuntimeCommand::StopSession {
                session_id,
                response,
            },
            "Session stop queue is unavailable",
        )?;
        result
            .recv()
            .context("Session stop worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Subscribes to a Session and reports a Workspace replacement prepared by the worker.
    pub fn subscribe_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<WorkspaceSwitchResult>> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            SessionRuntimeCommand::SubscribeSession {
                session_id,
                response,
            },
            "Session subscription queue is unavailable",
        )?;
        result
            .recv()
            .context("Session subscription worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Submits one shell turn to the active Thread.
    pub fn submit_shell_command(&self, command: String) -> Result<()> {
        self.try_send(
            SessionRuntimeCommand::SubmitShellCommand(command),
            "Shell submission queue is unavailable",
        )
    }

    /// Selects the model used by the active Session.
    pub fn select_model(&self, model: ModelRef) -> Result<()> {
        self.try_send(
            SessionRuntimeCommand::SelectModel(model),
            "Session model selection queue is unavailable",
        )
    }

    /// Refreshes the active Session subscription.
    pub fn refresh(&self) -> Result<()> {
        self.try_send(
            SessionRuntimeCommand::Refresh,
            "Session refresh queue is unavailable",
        )
    }

    /// Prepares and switches the worker to another Workspace.
    pub fn switch_workspace(&self, root: PathBuf) -> Result<WorkspaceSwitchResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            SessionRuntimeCommand::SwitchWorkspace { root, response },
            "Workspace switch queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace switch worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    fn try_send(&self, command: SessionRuntimeCommand, queue_error: &'static str) -> Result<()> {
        if !self.is_available() {
            return Err(anyhow!(SESSION_UNAVAILABLE_COMMAND_ERROR));
        }
        self.commands.try_send(command).context(queue_error)
    }
}

impl Drop for SessionRuntime {
    fn drop(&mut self) {
        self.available.store(false, Ordering::Release);
        let _ = self.commands.send(SessionRuntimeCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
