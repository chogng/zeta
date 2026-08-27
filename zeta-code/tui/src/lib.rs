//! Interactive terminal client for Zeta's App Server product boundary.

mod app;
mod client;
mod components;
mod features;
mod host;
mod keymap;
mod mouse;
mod terminal;
#[cfg(test)]
mod test_support;
mod ui;

use std::fmt;
use std::path::PathBuf;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::ShutdownError;
use zeta_app_server_client::TakeEventsError;
use zeta_app_server_protocol::protocol::common::AgentInteractionCapability;
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
use zeta_protocol::AgentInteractionKind;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

/// Declares the connection-local App Server capabilities required by the TUI.
///
/// The CLI host passes this value during `initialize` so App Server can select this connection as
/// the ephemeral owner for approval and structured user-input requests on subscribed Threads.
pub fn client_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        notifications: Some(true),
        agent_interactions: Some(AgentInteractionCapability {
            version: 1,
            kinds: vec![
                AgentInteractionKind::Approval,
                AgentInteractionKind::UserInput,
            ],
            dynamic_tools: None,
        }),
        browser: None,
        workspace_trust_host: None,
    }
}

/// Startup values owned by the CLI host rather than by the terminal UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiOptions {
    thread_title: String,
    display_workspace_root: PathBuf,
    host_workspace_root: PathBuf,
    host_file_search_root: Option<PathBuf>,
    keybindings_path: Option<PathBuf>,
    status_line_path: Option<PathBuf>,
    terminal_settings_path: Option<PathBuf>,
    recovery: Option<TuiRecoveryState>,
}

impl TuiOptions {
    pub fn new(thread_title: impl Into<String>) -> Self {
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            thread_title: thread_title.into(),
            display_workspace_root: workspace_root.clone(),
            host_workspace_root: workspace_root.clone(),
            host_file_search_root: Some(workspace_root),
            keybindings_path: None,
            status_line_path: None,
            terminal_settings_path: None,
            recovery: None,
        }
    }

    /// Uses `workspace_root` for display and bounded local host file operations.
    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        self.display_workspace_root = workspace_root.clone();
        self.host_workspace_root = workspace_root.clone();
        self.host_file_search_root = Some(workspace_root);
        self
    }

    /// Displays a Remote workspace without scanning it through the local host filesystem.
    ///
    /// Host-only output such as transcript export remains bounded to the local workspace root
    /// configured before this method is called. Remote path completion can be added later through
    /// an App Server-owned path-search contract.
    pub fn with_remote_workspace(mut self, remote_workspace_root: impl Into<PathBuf>) -> Self {
        self.display_workspace_root = remote_workspace_root.into();
        self.host_file_search_root = None;
        self
    }

    /// Enables host-local Zeta Code keybindings and terminal settings from the active profile.
    ///
    /// Product-scoped storage prevents desktop-only command identifiers from invalidating the
    /// TUI resource while preserving the shared JSON grammar and resolver precedence.
    pub fn with_profile_root(mut self, profile_root: impl Into<PathBuf>) -> Self {
        let product_root = profile_root.into().join("zeta-code");
        self.keybindings_path = Some(product_root.join("keybindings.json"));
        self.status_line_path = Some(product_root.join("statusline.json"));
        self.terminal_settings_path = Some(product_root.join("terminal.json"));
        self
    }

    /// Restores the durable Session and Thread selected before a transport loss.
    pub fn with_recovery(mut self, recovery: TuiRecoveryState) -> Self {
        self.recovery = Some(recovery);
        self
    }
}

/// Durable product identity returned to the CLI host after an App Server transport loss.
///
/// The state contains no transport handle, credential, pending request, or local command queue.
/// A host may establish a new connection and pass this state back through
/// [`TuiOptions::with_recovery`] to reload the canonical Thread snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiRecoveryState {
    session_id: SessionId,
    thread_id: ThreadId,
}

/// Host-owned Workspace reconnect requested by selecting a Session from another authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiWorkspaceReconnect {
    workspace_root: PathBuf,
    recovery: TuiRecoveryState,
}

impl TuiWorkspaceReconnect {
    pub(crate) fn new(workspace_root: PathBuf, recovery: TuiRecoveryState) -> Self {
        Self {
            workspace_root,
            recovery,
        }
    }

    /// Returns the canonical Workspace root recorded by the selected Session.
    pub fn workspace_root(&self) -> &std::path::Path {
        &self.workspace_root
    }

    /// Returns the Session and Thread identity to restore after reconnecting.
    pub fn recovery(&self) -> &TuiRecoveryState {
        &self.recovery
    }

    /// Splits the host reconnect target into its Workspace and durable recovery identity.
    pub fn into_parts(self) -> (PathBuf, TuiRecoveryState) {
        (self.workspace_root, self.recovery)
    }
}

/// Classifies why an initialized TUI connection reached its terminal boundary.
///
/// Product hosts may retry [`Self::Transport`] after establishing a new connection. A server
/// shutdown or protocol failure is terminal and must not be converted into a transport retry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiConnectionLossKind {
    /// The App Server transport stopped without a local shutdown request.
    Transport,
    /// The App Server session reported an orderly shutdown.
    ServerShutdown,
    /// The App Server stream violated the initialized protocol.
    Protocol,
}

impl TuiRecoveryState {
    pub(crate) fn new(session_id: SessionId, thread_id: ThreadId) -> Self {
        Self {
            session_id,
            thread_id,
        }
    }

    /// Returns the durable Session selected when the connection was lost.
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Returns the preferred durable Thread selected when the connection was lost.
    pub fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }

    pub(crate) fn into_parts(self) -> (SessionId, ThreadId) {
        (self.session_id, self.thread_id)
    }
}

/// Describes why the interactive terminal returned control to its host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuiExit {
    /// The user exited through an interactive TUI command or key binding.
    UserRequested,
    /// The host process received an operating-system termination request.
    TerminationRequested,
    /// The selected Session belongs to another Workspace and the product host must reconnect.
    WorkspaceReconnectRequested(TuiWorkspaceReconnect),
    /// The initialized App Server connection ended while the durable conversation remained.
    ConnectionLost {
        kind: TuiConnectionLossKind,
        recovery: TuiRecoveryState,
        reason: String,
    },
}

/// Failure to start or operate an interactive terminal session.
#[derive(Debug)]
pub enum TuiError {
    Client(ClientError),
    SessionEvents(TakeEventsError),
    Shutdown(ShutdownError),
    Terminal(std::io::Error),
}

impl fmt::Display for TuiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Client(error) => write!(formatter, "{error}"),
            Self::SessionEvents(error) => write!(formatter, "{error}"),
            Self::Shutdown(error) => write!(formatter, "{error}"),
            Self::Terminal(error) => write!(formatter, "terminal error: {error}"),
        }
    }
}

impl std::error::Error for TuiError {}

impl From<ClientError> for TuiError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}

impl From<std::io::Error> for TuiError {
    fn from(error: std::io::Error) -> Self {
        Self::Terminal(error)
    }
}

impl From<TakeEventsError> for TuiError {
    fn from(error: TakeEventsError) -> Self {
        Self::SessionEvents(error)
    }
}

impl From<ShutdownError> for TuiError {
    fn from(error: ShutdownError) -> Self {
        Self::Shutdown(error)
    }
}

/// Runs one interactive terminal session over an initialized App Server session.
///
/// The UI subscribes to the active product Thread and resynchronizes from canonical snapshots.
/// App Server notifications and terminal input independently wake the single-writer event loop.
/// A connection loss returns [`TuiExit::ConnectionLost`] without replaying pending or queued
/// commands, allowing the CLI host to reconnect and provide the durable recovery identity to a
/// later invocation. Other exits explicitly shut down the session before returning.
pub fn run(session: AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    app::run(session, options)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
