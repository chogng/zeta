use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use anyhow::anyhow;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::InProcessClientOptions;
use zeta_app_server_client::SessionStateMode;
use zeta_app_server_client::local_profile_root;
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::common::WorkspaceTrustHostCapability;
use zeta_remote::RemoteProfile;
use zeta_remote::SshHost;
use zeta_remote_connections::SshAppServerConnectionOptions;

/// Product-selected App Server location for one zeterm Agent session.
///
/// The native host decides whether a session is embedded or remote before the UI obtains an App
/// Server client. Neither the renderer nor the remote runtime receives local SSH credentials.
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
    /// Selects the existing embedded App Server composition for one local Workspace.
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

    /// Declares whether the current zeterm workspace picker can switch this target in place.
    pub(crate) const fn workspace_switch_support(&self) -> WorkspaceSwitchSupport {
        match self {
            Self::Local { .. } => WorkspaceSwitchSupport::Supported,
            Self::Ssh { .. } => WorkspaceSwitchSupport::Unsupported,
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
                let options = InProcessClientOptions::new(local_profile_root(), client_info)
                    .with_session_state_mode(SessionStateMode::Ephemeral)
                    .with_capabilities(ClientCapabilities {
                        workspace_trust_host: Some(WorkspaceTrustHostCapability { version: 1 }),
                        ..ClientCapabilities::default()
                    })
                    .with_workspace_root(workspace_root)
                    .with_discovered_product_services()?;
                AppServerSession::start_embedded(options)
                    .map_err(|error| anyhow!(error.to_string()))
            }
            Self::Ssh { connection, .. } => connection
                .connect(client_info, ClientCapabilities::default())
                .map_err(|error| anyhow!(error.to_string())),
        }
    }
}

/// Whether the existing local Workspace picker may call `workspace/switch` for this target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceSwitchSupport {
    Supported,
    Unsupported,
}
