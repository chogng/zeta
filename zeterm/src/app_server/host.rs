use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use anyhow::anyhow;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_client::local_profile_root;
use zeta_app_server_daemon::DAEMON_PATH_ENV;
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::common::WorkspaceTrustHostCapability;
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;
use zeta_remote_connections::SshAppServerConnectionOptions;

/// Product-owned App Server host context shared by Agent, Language, and Terminal adapters.
///
/// The host exposes one horizontal application backend contract. `Local` and `Remote` describe
/// how the App Server is reached; they do not create separate product APIs. Window state, panes,
/// commands, and Remote picker state remain owned by `zeterm` outside this context.
#[derive(Clone, Debug)]
pub(crate) struct AppServerHost {
    backend: AppServerBackend,
}

#[derive(Clone, Debug)]
enum AppServerBackend {
    Local {
        workspace_root: PathBuf,
    },
    Remote {
        connection: SshAppServerConnectionOptions,
        workspace_root: PathBuf,
    },
}

impl AppServerHost {
    /// Selects the profile-scoped local App Server authority for one Workspace.
    pub(crate) fn local(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            backend: AppServerBackend::Local {
                workspace_root: workspace_root.into(),
            },
        }
    }

    /// Selects an SSH-hosted App Server and an optional product-provided OpenSSH executable.
    pub(crate) fn remote_with_executable(
        profile: RemoteProfile,
        ssh_executable: Option<&Path>,
    ) -> Self {
        let workspace_root = PathBuf::from(profile.target().workspace().as_str());
        let mut connection = SshAppServerConnectionOptions::new(profile);
        if let Some(ssh_executable) = ssh_executable {
            connection = connection.with_ssh_executable(ssh_executable);
        }
        Self {
            backend: AppServerBackend::Remote {
                connection,
                workspace_root,
            },
        }
    }

    /// Returns whether the host delegates its App Server authority to SSH.
    pub(crate) const fn is_remote(&self) -> bool {
        matches!(&self.backend, AppServerBackend::Remote { .. })
    }

    /// Returns the authoritative Workspace path used in App Server requests and UI context.
    pub(crate) fn workspace_root(&self) -> &Path {
        match &self.backend {
            AppServerBackend::Local { workspace_root }
            | AppServerBackend::Remote { workspace_root, .. } => workspace_root,
        }
    }

    /// Retargets the same local profile or SSH host/runtime to another Workspace authority.
    pub(crate) fn with_workspace_root(&self, root: &Path) -> Result<Self> {
        match &self.backend {
            AppServerBackend::Local { .. } => Ok(Self::local(root)),
            AppServerBackend::Remote { connection, .. } => {
                let root = root
                    .to_str()
                    .ok_or_else(|| anyhow!("Remote Workspace path is not valid UTF-8"))?;
                let target = SshTarget::new(
                    connection.profile().target().host().clone(),
                    RemoteWorkspacePath::parse(root).map_err(|error| anyhow!(error.to_string()))?,
                );
                Ok(Self::remote_with_executable(
                    RemoteProfile::new(target, connection.profile().runtime().clone()),
                    Some(connection.ssh_executable()),
                ))
            }
        }
    }

    /// Returns the host-owned SSH inputs reusable by sibling native Remote capabilities.
    pub(crate) fn ssh_transport(&self) -> Option<(&SshHost, &Path)> {
        match &self.backend {
            AppServerBackend::Local { .. } => None,
            AppServerBackend::Remote { connection, .. } => Some((
                connection.profile().target().host(),
                connection.ssh_executable(),
            )),
        }
    }

    /// Returns the Remote App Server connection backend when this host is remote.
    pub(crate) fn remote_connection(&self) -> Option<&SshAppServerConnectionOptions> {
        match &self.backend {
            AppServerBackend::Local { .. } => None,
            AppServerBackend::Remote { connection, .. } => Some(connection),
        }
    }

    /// Opens the selected backend and performs the canonical initialize/schema handshake.
    pub(crate) fn start(&self) -> Result<AppServerSession> {
        let client_info = ClientInfo {
            name: "zeterm".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        };
        match &self.backend {
            AppServerBackend::Local { workspace_root } => {
                let executable = std::env::current_exe()
                    .map_err(|error| anyhow!("could not resolve zeterm executable: {error}"))?;
                let daemon_executable = development_daemon_executable(&executable);
                let command = local_app_server_command(
                    executable,
                    local_profile_root(),
                    workspace_root,
                    daemon_executable,
                );
                AppServerSession::start_stdio(command, client_info, local_client_capabilities())
                    .map_err(|error| anyhow!(error.to_string()))
            }
            AppServerBackend::Remote { connection, .. } => connection
                .connect(client_info, ClientCapabilities::default())
                .map_err(|error| anyhow!(error.to_string())),
        }
    }
}

pub(crate) fn local_app_server_command(
    executable: PathBuf,
    profile_root: PathBuf,
    workspace_root: &Path,
    daemon_executable: Option<PathBuf>,
) -> StdioAppServerCommand {
    let command = StdioAppServerCommand::new(executable)
        .with_argument("app-server")
        .with_argument("connect")
        .with_environment_variable("ZETA_PROFILE_ROOT", profile_root.into_os_string())
        .with_environment_variable(
            "ZETA_WORKSPACE_ROOT",
            workspace_root.as_os_str().to_os_string(),
        );
    match daemon_executable {
        Some(daemon_executable) => {
            command.with_environment_variable(DAEMON_PATH_ENV, daemon_executable.into_os_string())
        }
        None => command,
    }
}

fn development_daemon_executable(zeterm_executable: &Path) -> Option<PathBuf> {
    if cfg!(debug_assertions) && std::env::var_os(DAEMON_PATH_ENV).is_none() {
        Some(zeterm_executable.to_path_buf())
    } else {
        None
    }
}

fn local_client_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        workspace_trust_host: Some(WorkspaceTrustHostCapability { version: 1 }),
        ..ClientCapabilities::default()
    }
}
