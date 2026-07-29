//! Interactive terminal client for Zeta's App Server product boundary.

mod app;
mod client;
mod components;
mod features;
mod host;
mod terminal;
mod ui;

use std::fmt;
use std::path::PathBuf;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::ShutdownError;
use zeta_app_server_client::TakeEventsError;

/// Startup values owned by the CLI host rather than by the terminal UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiOptions {
    thread_title: String,
    workspace_root: PathBuf,
}

impl TuiOptions {
    pub fn new(thread_title: impl Into<String>) -> Self {
        Self {
            thread_title: thread_title.into(),
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Uses `workspace_root` as the bounded source for `@file` mention candidates.
    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace_root = workspace_root.into();
        self
    }
}

/// Describes why the interactive terminal returned control to its host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiExit {
    UserRequested,
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
/// App Server notifications and terminal input independently wake the single-writer event loop,
/// and the session is explicitly shut down before control returns to the CLI.
pub fn run(session: AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    app::run(session, options)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
