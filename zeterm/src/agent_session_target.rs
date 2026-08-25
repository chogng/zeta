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

/// Product-selected App Server location for one zeterm Agent session.
///
/// The native host decides whether a session uses the profile-scoped local authority or a remote
/// authority before the UI obtains an App Server client. Neither the renderer nor the remote
/// runtime receives local SSH credentials.
#[derive(Clone, Debug)]
pub(crate) enum AgentSessionTarget {
    Local {
        workspace_root: PathBuf,
    },
    Ssh {
        connection: SshAppServerConnectionOptions,
        workspace_root: PathBuf,
    },
}

impl AgentSessionTarget {
    /// Selects the profile-scoped App Server authority for one local Workspace.
    pub(crate) fn local(workspace_root: impl Into<PathBuf>) -> Self {
        Self::Local {
            workspace_root: workspace_root.into(),
        }
    }

    /// Selects an SSH-hosted App Server and an optional product-provided OpenSSH executable.
    pub(crate) fn ssh_with_executable(
        profile: RemoteProfile,
        ssh_executable: Option<&Path>,
    ) -> Self {
        let workspace_root = PathBuf::from(profile.target().workspace().as_str());
        let mut connection = SshAppServerConnectionOptions::new(profile);
        if let Some(ssh_executable) = ssh_executable {
            connection = connection.with_ssh_executable(ssh_executable);
        }
        Self::Ssh {
            connection,
            workspace_root,
        }
    }

    /// Returns whether this target delegates filesystem and terminal authority to an SSH host.
    pub(crate) const fn is_remote(&self) -> bool {
        matches!(self, Self::Ssh { .. })
    }

    /// Returns the authoritative Workspace path used in Session titles and App Server requests.
    pub(crate) fn workspace_root(&self) -> &Path {
        match self {
            Self::Local { workspace_root } | Self::Ssh { workspace_root, .. } => workspace_root,
        }
    }

    /// Retargets the same local Profile or SSH host/runtime to another Workspace authority.
    pub(crate) fn with_workspace_root(&self, root: &Path) -> Result<Self> {
        match self {
            Self::Local { .. } => Ok(Self::local(root)),
            Self::Ssh { connection, .. } => {
                let root = root
                    .to_str()
                    .ok_or_else(|| anyhow!("Remote Workspace path is not valid UTF-8"))?;
                let target = SshTarget::new(
                    connection.profile().target().host().clone(),
                    RemoteWorkspacePath::parse(root).map_err(|error| anyhow!(error.to_string()))?,
                );
                Ok(Self::ssh_with_executable(
                    RemoteProfile::new(target, connection.profile().runtime().clone()),
                    Some(connection.ssh_executable()),
                ))
            }
        }
    }

    /// Returns the host-owned SSH inputs reusable by sibling native Remote capabilities.
    pub(crate) fn ssh_transport(&self) -> Option<(&SshHost, &Path)> {
        match self {
            Self::Local { .. } => None,
            Self::Ssh { connection, .. } => Some((
                connection.profile().target().host(),
                connection.ssh_executable(),
            )),
        }
    }

    /// Opens the target and performs the canonical initialize/schema handshake.
    pub(crate) fn start(&self) -> Result<AppServerSession> {
        let client_info = ClientInfo {
            name: "zeterm".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        };
        match self {
            Self::Local { workspace_root } => {
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
            Self::Ssh { connection, .. } => connection
                .connect(client_info, ClientCapabilities::default())
                .map_err(|error| anyhow!(error.to_string())),
        }
    }
}

pub(super) fn local_app_server_command(
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
