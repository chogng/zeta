use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use zeta_app_server_protocol::protocol::config::ConfigCommandResult;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigDto;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryEntry;
use zeta_app_server_protocol::protocol::git::GitBranchDto;
use zeta_app_server_protocol::protocol::git::GitTextDiffResult;
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;
use zeta_protocol::ModelRef;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadUpdateEnvelope;
use zeta_text_file::TextFileDiskVersion;
use zeta_text_file::TextFileSaveRequest;
use zeta_text_file::TextFileSnapshot;

/// The default number of commands that may wait for the worker.
pub const DEFAULT_COMMAND_QUEUE_CAPACITY: usize = 32;

/// Error returned when a command arrives while the worker has no live connection.
pub const AGENT_UNAVAILABLE_COMMAND_ERROR: &str =
    "Agent session is not connected; the command was not sent";

/// Maximum time spent trying to recover a lost remote connection.
pub const RECONNECT_WINDOW: Duration = Duration::from_secs(30);

const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(250);
/// Maximum delay between two remote reconnect attempts.
pub const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// The result type used by request/response commands.
pub type CommandResult<T> = std::result::Result<T, String>;

/// Product-owned correlation id for one Session Tab activation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionSwitchId(u64);

impl SessionSwitchId {
    /// Creates an id from the host's monotonically increasing counter.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the counter value for host-side diagnostics.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The result prepared before the host reconnects to another workspace.
#[derive(Debug)]
pub struct WorkspaceSwitchResult {
    /// The workspace root that the next connection should use.
    pub root: PathBuf,
    /// The Git snapshot read while preparing the next connection, if available.
    pub git: Option<GitTextDiffResult>,
}

/// Events emitted by the Agent Session worker.
#[derive(Debug)]
pub enum AgentSessionEvent {
    /// Catalog data needed by the composer.
    Catalog {
        /// Slash commands advertised by the server.
        slash_commands: Vec<SlashCommandDefinition>,
        /// Models advertised by the server.
        models: Vec<ModelCatalogEntry>,
    },
    /// Current language-service configuration.
    Configuration(ConfigReadResult),
    /// Session list for the current workspace.
    SessionCatalog(Vec<Session>),
    /// Authoritative snapshot for the active thread.
    Snapshot {
        /// Session containing the active thread.
        session: Session,
        /// Current thread state.
        thread: Thread,
        /// Optional activation that caused this snapshot.
        switch_id: Option<SessionSwitchId>,
    },
    /// One incremental Thread update from the server.
    Update(Box<ThreadUpdateEnvelope>),
    /// Latest Git diff snapshot, or `None` when the workspace has no repository.
    GitSnapshot(Option<GitTextDiffResult>),
    /// Filesystem changes reported by the server.
    FilesChanged(FsChanged),
    /// A worker or transport error.
    Error(String),
    /// The worker has stopped and will emit no more events.
    Closed,
}

/// Commands accepted by the Agent Session worker.
pub enum AgentSessionCommand {
    /// Create and activate a new Session.
    CreateSession,
    /// Stop a Session and return the server result.
    StopSession {
        /// Session to stop.
        session_id: SessionId,
        /// Completion channel.
        response: SyncSender<CommandResult<()>>,
    },
    /// Activate a Session and optionally prepare a workspace reconnect.
    ActivateSession {
        /// Session to activate.
        session_id: SessionId,
        /// Host-owned activation correlation id.
        switch_id: SessionSwitchId,
        /// Completion channel.
        response: SyncSender<CommandResult<Option<WorkspaceSwitchResult>>>,
    },
    /// Submit a text turn to the active Thread.
    SubmitAgentMessage(String),
    /// Submit a shell turn to the active Thread.
    SubmitShellCommand(String),
    /// Change the model for the active Session.
    SelectModel(ModelRef),
    /// Refresh the active Session and Thread snapshot.
    Refresh,
    /// Refresh the Git snapshot.
    RefreshGit,
    /// Read directory entries from the current workspace.
    ReadDirectory {
        /// Directory path relative to the workspace.
        path: PathBuf,
        /// Completion channel.
        response: SyncSender<CommandResult<Vec<FsReadDirectoryEntry>>>,
    },
    /// Read a text file from the current workspace.
    ReadFile {
        /// File path relative to the workspace.
        path: PathBuf,
        /// Completion channel.
        response: SyncSender<CommandResult<TextFileSnapshot>>,
    },
    /// Save a text file in the current workspace.
    WriteFile {
        /// Save request including the expected disk version.
        request: TextFileSaveRequest,
        /// Completion channel.
        response: SyncSender<CommandResult<TextFileDiskVersion>>,
    },
    /// Read local Git branches.
    ListGitBranches(SyncSender<CommandResult<Vec<GitBranchDto>>>),
    /// Switch the current Git branch and return its new diff snapshot.
    SwitchGitBranch {
        /// Branch name to select.
        name: String,
        /// Completion channel.
        response: SyncSender<CommandResult<GitTextDiffResult>>,
    },
    /// Prepare a connection to another workspace.
    SwitchWorkspace {
        /// Workspace root to use for the next connection.
        root: PathBuf,
        /// Completion channel.
        response: SyncSender<CommandResult<WorkspaceSwitchResult>>,
    },
    /// Persist one language-server configuration change.
    ConfigureLanguageServer {
        /// Revision the host read before opening the form.
        expected_revision: u64,
        /// Server id being configured.
        server_id: String,
        /// New server configuration.
        config: LanguageServerConfigDto,
        /// Completion channel.
        response: SyncSender<CommandResult<ConfigCommandResult>>,
    },
    /// Remove one language-server configuration.
    RemoveLanguageServerConfiguration {
        /// Revision the host read before opening the form.
        expected_revision: u64,
        /// Server id being removed.
        server_id: String,
        /// Completion channel.
        response: SyncSender<CommandResult<ConfigCommandResult>>,
    },
    /// Ask the worker to stop after draining no further work.
    Shutdown,
}

/// Sender half of the bounded Agent Session command queue.
pub type AgentSessionCommandSender = SyncSender<AgentSessionCommand>;

/// Receiver half of the bounded Agent Session command queue.
pub type AgentSessionCommandReceiver = Receiver<AgentSessionCommand>;

/// Creates the standard-sized Agent Session command queue.
pub fn command_channel() -> (AgentSessionCommandSender, AgentSessionCommandReceiver) {
    command_channel_with_capacity(DEFAULT_COMMAND_QUEUE_CAPACITY)
}

/// Creates a bounded Agent Session command queue for a host or a focused test.
pub fn command_channel_with_capacity(
    capacity: usize,
) -> (AgentSessionCommandSender, AgentSessionCommandReceiver) {
    mpsc::sync_channel(capacity)
}

/// Returns the exponential-backoff delay for a reconnect attempt.
pub fn reconnect_delay(attempt: usize) -> Duration {
    let multiplier = 1_u32 << (attempt.min(31) as u32);
    (INITIAL_RECONNECT_DELAY * multiplier).min(MAX_RECONNECT_DELAY)
}

/// Returns a reconnect delay only when it fits inside the recovery window.
pub fn reconnect_delay_within_window(elapsed: Duration, attempt: usize) -> Option<Duration> {
    let remaining = RECONNECT_WINDOW.checked_sub(elapsed)?;
    let delay = reconnect_delay(attempt);
    (delay <= remaining).then_some(delay)
}

/// Rejects queued commands while disconnected and completes request/response commands with a
/// stable error. Returns `true` when the worker should stop instead of recovering.
pub fn reject_disconnected_command(command: AgentSessionCommand) -> bool {
    match command {
        AgentSessionCommand::Shutdown => return true,
        AgentSessionCommand::ReadDirectory { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::ReadFile { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::WriteFile { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::ListGitBranches(response) => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::SwitchGitBranch { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::SwitchWorkspace { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::ActivateSession { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::StopSession { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::ConfigureLanguageServer { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::RemoveLanguageServerConfiguration { response, .. } => {
            let _ = response.send(disconnected_command_error());
        }
        AgentSessionCommand::CreateSession
        | AgentSessionCommand::SubmitAgentMessage(_)
        | AgentSessionCommand::SubmitShellCommand(_)
        | AgentSessionCommand::SelectModel(_)
        | AgentSessionCommand::Refresh
        | AgentSessionCommand::RefreshGit => {}
    }
    false
}

fn disconnected_command_error<T>() -> CommandResult<T> {
    Err(AGENT_UNAVAILABLE_COMMAND_ERROR.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_queue_is_bounded() {
        let (sender, receiver) = command_channel_with_capacity(1);
        sender.send(AgentSessionCommand::Refresh).unwrap();

        assert!(sender.try_send(AgentSessionCommand::Refresh).is_err());
        assert!(matches!(
            receiver.recv().unwrap(),
            AgentSessionCommand::Refresh
        ));
    }

    #[test]
    fn switch_id_round_trips_without_host_types() {
        let id = SessionSwitchId::new(42);
        assert_eq!(id.get(), 42);
    }

    #[test]
    fn remote_reconnect_backoff_is_bounded() {
        assert_eq!(reconnect_delay(0), Duration::from_millis(250));
        assert_eq!(reconnect_delay(1), Duration::from_millis(500));
        assert_eq!(reconnect_delay(3), Duration::from_secs(2));
        assert_eq!(reconnect_delay(32), MAX_RECONNECT_DELAY);
    }

    #[test]
    fn remote_reconnect_never_schedules_past_its_recovery_window() {
        assert_eq!(
            reconnect_delay_within_window(Duration::ZERO, 0),
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            reconnect_delay_within_window(RECONNECT_WINDOW - Duration::from_secs(2), 8),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            reconnect_delay_within_window(RECONNECT_WINDOW - Duration::from_millis(100), 0),
            None
        );
        assert_eq!(reconnect_delay_within_window(RECONNECT_WINDOW, 0), None);
    }

    #[test]
    fn disconnected_request_is_completed_without_replay() {
        let (response, result) = mpsc::sync_channel(1);
        assert!(!reject_disconnected_command(
            AgentSessionCommand::ReadDirectory {
                path: PathBuf::from("src"),
                response,
            }
        ));
        assert_eq!(
            result.recv().unwrap(),
            Err(AGENT_UNAVAILABLE_COMMAND_ERROR.to_owned())
        );
        assert!(reject_disconnected_command(AgentSessionCommand::Shutdown));
    }
}
