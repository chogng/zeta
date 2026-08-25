//! Profile-scoped App Server daemon lifecycle, runtime, and stdio proxy.

mod client;
mod daemon;
mod endpoint;
mod process;
mod registry;
mod wire;

use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;
use zeta_app_server_client::local_profile_root;

/// Environment variable that selects the daemon executable used by a client host.
pub const DAEMON_PATH_ENV: &str = "ZETA_APP_SERVER_DAEMON_PATH";

/// Internal argument that selects the daemon role in a product executable that embeds this crate.
pub const DAEMON_PROCESS_ARGUMENT: &str = "--zeta-app-server-daemon-process";

/// Workspace trust source attached to one daemon connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceTrustSource {
    /// Trust was resolved by the product host and passed explicitly.
    HostConfiguration,
    /// Trust is resolved from the shared user configuration.
    UserConfig,
}

/// Profile, Workspace, trust, and product-service inputs for one daemon connection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionOptions {
    profile_root: PathBuf,
    workspace_root: Option<PathBuf>,
    workspace_trust_source: WorkspaceTrustSource,
    product_services: Option<PathBuf>,
}

impl ConnectionOptions {
    /// Creates the explicit inputs carried in a daemon connection prelude.
    pub fn new(
        profile_root: impl Into<PathBuf>,
        workspace_root: Option<PathBuf>,
        workspace_trust_source: WorkspaceTrustSource,
        product_services: Option<PathBuf>,
    ) -> Self {
        Self {
            profile_root: profile_root.into(),
            workspace_root,
            workspace_trust_source,
            product_services,
        }
    }

    /// Returns the shared local profile root.
    pub fn profile_root(&self) -> &Path {
        &self.profile_root
    }

    /// Returns the explicitly selected Workspace root, when present.
    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    /// Returns the trust source for the selected Workspace.
    pub fn workspace_trust_source(&self) -> WorkspaceTrustSource {
        self.workspace_trust_source
    }

    /// Returns the optional product-services manifest for this connection.
    pub fn product_services(&self) -> Option<&Path> {
        self.product_services.as_deref()
    }
}

/// One machine-readable daemon lifecycle operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleCommand {
    /// Starts the daemon when it is not already ready.
    Start,
    /// Stops a managed daemon and starts a new process generation.
    Restart,
    /// Gracefully stops the managed daemon.
    Stop,
    /// Probes the running daemon and its App Server initialize contract.
    Version,
}

/// Stable lifecycle result status written by product-neutral command hosts.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleStatus {
    /// The existing daemon was already initialized and compatible.
    AlreadyRunning,
    /// A new daemon process was started and initialized.
    Started,
    /// The previous daemon was stopped and a new generation initialized.
    Restarted,
    /// A running daemon was stopped.
    Stopped,
    /// No daemon is currently running for the profile.
    NotRunning,
    /// The daemon and App Server initialize contract are healthy.
    Running,
}

/// Machine-readable lifecycle output for one profile-scoped daemon.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleOutput {
    /// Operation-specific lifecycle status.
    pub status: LifecycleStatus,
    /// Managed daemon process identifier, when running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Per-process identity published in the private daemon state directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// Daemon crate version.
    pub daemon_version: String,
    /// Profile-scoped control and App Server socket.
    pub endpoint_path: PathBuf,
    /// Bounded daemon stdout/stderr log.
    pub log_path: PathBuf,
    /// Initialized App Server name, when running and compatible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_server_name: Option<String>,
    /// Initialized App Server schema identity, when running and compatible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_hash: Option<String>,
}

/// Runs one serialized lifecycle operation for the selected profile.
#[cfg(any(unix, windows))]
pub fn run_lifecycle(
    command: LifecycleCommand,
    options: ConnectionOptions,
    daemon_executable: &Path,
) -> Result<LifecycleOutput, String> {
    client::run_lifecycle(command, options, daemon_executable)
}

/// Connects stdio to a ready daemon, starting and probing it first when necessary.
#[cfg(any(unix, windows))]
pub fn connect(options: ConnectionOptions, daemon_executable: &Path) -> Result<(), String> {
    client::connect(options, daemon_executable)
}

/// Runs the profile authority process until stopped or idle.
#[cfg(any(unix, windows))]
pub fn serve(profile_root: impl Into<PathBuf>) -> Result<(), String> {
    daemon::serve(ConnectionOptions::new(
        profile_root,
        None,
        WorkspaceTrustSource::HostConfiguration,
        None,
    ))
}

/// Returns the profile-scoped daemon endpoint path used for diagnostics and integration tests.
#[cfg(any(unix, windows))]
pub fn daemon_endpoint_path(profile_root: &Path) -> Result<PathBuf, String> {
    endpoint::EndpointPaths::prepare(profile_root).map(|endpoint| endpoint.socket)
}

/// Runs the daemon process entrypoint using the local profile selected by the environment.
pub fn run_from_environment(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    if !arguments.is_empty() && arguments.as_slice() != [DAEMON_PROCESS_ARGUMENT] {
        return Err("usage: zeta-app-server-daemon".into());
    }
    serve(local_profile_root())
}

#[cfg(not(any(unix, windows)))]
pub fn run_lifecycle(
    _command: LifecycleCommand,
    _options: ConnectionOptions,
    _daemon_executable: &Path,
) -> Result<LifecycleOutput, String> {
    Err("Local App Server daemon requires Unix-domain socket support".into())
}

#[cfg(not(any(unix, windows)))]
pub fn connect(_options: ConnectionOptions, _daemon_executable: &Path) -> Result<(), String> {
    Err("Local App Server daemon requires Unix-domain socket support".into())
}

#[cfg(not(any(unix, windows)))]
pub fn serve(_profile_root: impl Into<PathBuf>) -> Result<(), String> {
    Err("Local App Server daemon requires Unix-domain socket support".into())
}

/// Reports that this target does not support a local daemon endpoint.
#[cfg(not(any(unix, windows)))]
pub fn daemon_endpoint_path(_profile_root: &Path) -> Result<PathBuf, String> {
    Err("Local App Server daemon requires Unix-domain socket support".into())
}

#[cfg(test)]
#[path = "app_server_daemon_tests.rs"]
mod tests;
