//! App-side Agent Session worker and its typed product boundary.
//!
//! This crate owns the App Server connection, subscription, command queue, file and Git requests,
//! and reconnect lifecycle. The product host supplies only a connection target and an event sink;
//! window, focus, panes, and rendering stay outside this crate.

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
use zeta_app_server_protocol::protocol::config::ConfigCommandResult;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigDto;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryEntry;
use zeta_app_server_protocol::protocol::git::GitBranchDto;
use zeta_app_server_protocol::protocol::git::GitTextDiffResult;
use zeta_protocol::ModelRef;
use zeta_protocol::SessionId;
use zeta_text_file::TextFileDiskVersion;
use zeta_text_file::TextFileSaveRequest;
use zeta_text_file::TextFileSnapshot;

mod contract;
mod worker;

pub use contract::AGENT_UNAVAILABLE_COMMAND_ERROR;
pub use contract::AgentSessionCommand;
pub use contract::AgentSessionCommandReceiver;
pub use contract::AgentSessionCommandSender;
pub use contract::AgentSessionEvent;
pub use contract::CommandResult;
pub use contract::DEFAULT_COMMAND_QUEUE_CAPACITY;
pub use contract::MAX_RECONNECT_DELAY;
pub use contract::RECONNECT_WINDOW;
pub use contract::SessionSwitchId;
pub use contract::WorkspaceSwitchResult;
pub use contract::command_channel;
pub use contract::command_channel_with_capacity;
pub use contract::reconnect_delay;
pub use contract::reconnect_delay_within_window;
pub use contract::reject_disconnected_command;

/// Connection authority supplied by the product host.
///
/// Implementations preserve their Local or Remote transport identity when retargeted to another
/// Workspace. The Agent worker uses this contract without importing product composition types.
pub trait AgentSessionTarget: Send + Sync {
    /// Returns whether transport loss should use the bounded reconnect policy.
    fn is_remote(&self) -> bool;

    /// Returns the Workspace root represented by this target.
    fn workspace_root(&self) -> &Path;

    /// Creates the same connection authority retargeted to another Workspace.
    fn retarget(&self, root: &Path) -> CommandResult<Box<dyn AgentSessionTarget>>;

    /// Opens and initializes one App Server protocol session.
    fn start(&self) -> CommandResult<zeta_app_server_client::AppServerSession>;
}

type AgentSessionEventSink = Arc<dyn Fn(AgentSessionEvent) -> CommandResult<()> + Send + Sync>;
/// Running Agent Session worker handle used by the product host.
pub struct AgentSession {
    available: Arc<AtomicBool>,
    commands: SyncSender<AgentSessionCommand>,
    worker: Option<JoinHandle<()>>,
}

impl AgentSession {
    /// Starts one worker for the supplied App Server target and product event sink.
    pub fn spawn<T, F>(target: T, event_sink: F) -> Result<Self>
    where
        T: AgentSessionTarget + 'static,
        F: Fn(AgentSessionEvent) -> CommandResult<()> + Send + Sync + 'static,
    {
        let (commands, command_receiver) = command_channel();
        let available = Arc::new(AtomicBool::new(false));
        let worker_availability = Arc::clone(&available);
        let event_sink: AgentSessionEventSink = Arc::new(event_sink);
        let worker = thread::Builder::new()
            .name("app-agent-session".into())
            .spawn(move || {
                worker::run_agent_session(
                    event_sink,
                    command_receiver,
                    Box::new(target),
                    worker_availability,
                )
            })
            .context("could not start Agent Session worker")?;
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
            AgentSessionCommand::SubmitAgentMessage(text),
            "Agent submission queue is unavailable",
        )
    }

    /// Creates and activates a Session in the current Workspace.
    pub fn create_session(&self) -> Result<()> {
        self.try_send(
            AgentSessionCommand::CreateSession,
            "Agent session creation queue is unavailable",
        )
    }

    /// Stops an active Session.
    pub fn stop_session(&self, session_id: SessionId) -> Result<()> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::StopSession {
                session_id,
                response,
            },
            "Agent session stop queue is unavailable",
        )?;
        result
            .recv()
            .context("Agent session stop worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Activates a Session and reports a Workspace replacement prepared by the worker.
    pub fn activate_session(
        &self,
        session_id: SessionId,
        switch_id: SessionSwitchId,
    ) -> Result<Option<WorkspaceSwitchResult>> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ActivateSession {
                session_id,
                switch_id,
                response,
            },
            "Agent session activation queue is unavailable",
        )?;
        result
            .recv()
            .context("Agent session activation worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Submits one shell turn to the active Thread.
    pub fn submit_shell_command(&self, command: String) -> Result<()> {
        self.try_send(
            AgentSessionCommand::SubmitShellCommand(command),
            "Shell submission queue is unavailable",
        )
    }

    /// Selects the model used by the active Session.
    pub fn select_model(&self, model: ModelRef) -> Result<()> {
        self.try_send(
            AgentSessionCommand::SelectModel(model),
            "Agent model selection queue is unavailable",
        )
    }

    /// Refreshes the active Session subscription.
    pub fn refresh(&self) -> Result<()> {
        self.try_send(
            AgentSessionCommand::Refresh,
            "Agent refresh queue is unavailable",
        )
    }

    /// Refreshes the Git projection.
    pub fn refresh_git(&self) -> Result<()> {
        self.try_send(
            AgentSessionCommand::RefreshGit,
            "Git refresh queue is unavailable",
        )
    }

    /// Reads a Workspace directory through App Server.
    pub fn read_directory(&self, path: PathBuf) -> Result<Vec<FsReadDirectoryEntry>> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ReadDirectory { path, response },
            "Workspace directory query queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace directory query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Reads one stable text-file snapshot through App Server.
    pub fn read_file(&self, path: PathBuf) -> Result<TextFileSnapshot> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ReadFile { path, response },
            "Workspace file query queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace file query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Saves one text file through App Server.
    pub fn write_file(&self, request: TextFileSaveRequest) -> Result<TextFileDiskVersion> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::WriteFile { request, response },
            "Workspace file mutation queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace file mutation worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Lists local Git branches through App Server.
    pub fn local_branches(&self) -> Result<Vec<GitBranchDto>> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ListGitBranches(response),
            "Git branch query queue is unavailable",
        )?;
        result
            .recv()
            .context("Git branch query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Switches the current Git branch through App Server.
    pub fn switch_git_branch(&self, name: String) -> Result<GitTextDiffResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::SwitchGitBranch { name, response },
            "Git branch mutation queue is unavailable",
        )?;
        result
            .recv()
            .context("Git branch mutation worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Prepares and switches the worker to another Workspace.
    pub fn switch_workspace(&self, root: PathBuf) -> Result<WorkspaceSwitchResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::SwitchWorkspace { root, response },
            "Workspace switch queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace switch worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Persists one language-server configuration.
    pub fn configure_language_server(
        &self,
        expected_revision: u64,
        server_id: String,
        config: LanguageServerConfigDto,
    ) -> Result<ConfigCommandResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ConfigureLanguageServer {
                expected_revision,
                server_id,
                config,
                response,
            },
            "Language server configuration queue is unavailable",
        )?;
        result
            .recv()
            .context("Language server configuration worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    /// Removes one language-server configuration.
    pub fn remove_language_server_configuration(
        &self,
        expected_revision: u64,
        server_id: String,
    ) -> Result<ConfigCommandResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::RemoveLanguageServerConfiguration {
                expected_revision,
                server_id,
                response,
            },
            "Language server configuration queue is unavailable",
        )?;
        result
            .recv()
            .context("Language server configuration worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    fn try_send(&self, command: AgentSessionCommand, queue_error: &'static str) -> Result<()> {
        if !self.is_available() {
            return Err(anyhow!(AGENT_UNAVAILABLE_COMMAND_ERROR));
        }
        self.commands.try_send(command).context(queue_error)
    }
}

impl Drop for AgentSession {
    fn drop(&mut self) {
        self.available.store(false, Ordering::Release);
        let _ = self.commands.send(AgentSessionCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
