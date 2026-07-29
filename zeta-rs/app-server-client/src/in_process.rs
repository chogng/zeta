use crate::AppServerClient;
use crate::ClientError;
use crate::JsonRpcTransport;
use std::path::PathBuf;
use std::sync::Arc;
use zeta_app_server::AppServer;
use zeta_app_server::BuiltInSkillRoot;
use zeta_app_server::ConnectionState;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::SlashCommandCatalog;
use zeta_app_server::open_local_app_server;
use zeta_app_server_protocol::protocol::common::{ClientCapabilities, ClientInfo};
use zeta_app_server_protocol::protocol::initialize::InitializeParams;
use zeta_app_server_protocol::schema_hash;

/// Startup inputs for an embedded App Server connection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InProcessClientOptions {
    pub state_root: PathBuf,
    pub workspace_root: Option<PathBuf>,
    pub client_info: ClientInfo,
    pub capabilities: ClientCapabilities,
    pub slash_commands: SlashCommandCatalog,
    pub built_in_skills: BuiltInSkillRoot,
}

impl InProcessClientOptions {
    pub fn new(state_root: impl Into<PathBuf>, client_info: ClientInfo) -> Self {
        Self {
            state_root: state_root.into(),
            workspace_root: None,
            client_info,
            capabilities: ClientCapabilities::default(),
            slash_commands: SlashCommandCatalog::default(),
            built_in_skills: BuiltInSkillRoot::AutoDetect,
        }
    }

    pub fn with_capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_slash_command_catalog(mut self, slash_commands: SlashCommandCatalog) -> Self {
        self.slash_commands = slash_commands;
        self
    }

    /// Enables local filesystem and shell tools under one Workspace root.
    pub fn with_workspace_root(mut self, workspace: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(workspace.into());
        self
    }

    pub fn with_built_in_skill_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.built_in_skills = BuiltInSkillRoot::Explicit(root.into());
        self
    }

    pub fn without_built_in_skills(mut self) -> Self {
        self.built_in_skills = BuiltInSkillRoot::Unavailable;
        self
    }
}

/// In-memory transport that still exercises the versioned JSON-RPC dispatcher.
pub struct InProcessTransport {
    server: Arc<AppServer>,
    connection: ConnectionState,
    notifications: Vec<String>,
}

impl InProcessTransport {
    /// Creates an embedded transport that routes every request through the App Server dispatcher.
    ///
    /// Hosts that provide their own composition root can use this instead of the local filesystem
    /// composition used by [`start_in_process_client`].
    pub fn from_server(server: AppServer) -> Self {
        Self::from_shared_server(Arc::new(server))
    }

    /// Creates one logical connection to a shared embedded App Server composition root.
    pub fn from_shared_server(server: Arc<AppServer>) -> Self {
        let connection = server.connection();
        Self {
            server,
            connection,
            notifications: Vec::new(),
        }
    }
}

/// Shared embedded App Server composition that can open multiple isolated logical connections.
#[derive(Clone)]
pub struct InProcessAppServer {
    pub(crate) server: Arc<AppServer>,
    pub(crate) client_info: ClientInfo,
    pub(crate) capabilities: ClientCapabilities,
}

impl InProcessAppServer {
    /// Opens and initializes one typed client connection to the shared App Server.
    pub fn connect(&self) -> Result<AppServerClient<InProcessTransport>, ClientError> {
        initialize_client(
            InProcessTransport::from_shared_server(self.server.clone()),
            self.client_info.clone(),
            self.capabilities.clone(),
        )
    }
}

impl JsonRpcTransport for InProcessTransport {
    fn round_trip(&mut self, request: &str) -> Result<String, ClientError> {
        let response = self.server.handle_json(&mut self.connection, request);
        self.notifications
            .extend(self.server.drain_notifications(&mut self.connection));
        Ok(response)
    }

    fn drain_notifications(&mut self) -> Result<Vec<String>, ClientError> {
        self.notifications
            .extend(self.server.drain_notifications(&mut self.connection));
        Ok(std::mem::take(&mut self.notifications))
    }
}

/// Opens, initializes, and schema-checks an embedded App Server client.
pub fn start_in_process_client(
    options: InProcessClientOptions,
) -> Result<AppServerClient<InProcessTransport>, ClientError> {
    open_in_process_app_server(options)?.connect()
}

/// Opens one embedded App Server composition that may serve multiple client connections.
pub fn open_in_process_app_server(
    options: InProcessClientOptions,
) -> Result<InProcessAppServer, ClientError> {
    let mut server_options = LocalAppServerOptions::new(options.state_root)
        .with_slash_command_catalog(options.slash_commands);
    server_options.built_in_skills = options.built_in_skills;
    if let Some(workspace_root) = options.workspace_root {
        server_options = server_options.with_workspace_root(workspace_root);
    }
    let server = open_local_app_server(server_options)
        .map_err(|error| ClientError::Transport(error.to_string()))?;
    Ok(InProcessAppServer {
        server: Arc::new(server),
        client_info: options.client_info,
        capabilities: options.capabilities,
    })
}

fn initialize_client(
    transport: InProcessTransport,
    client_info: ClientInfo,
    capabilities: ClientCapabilities,
) -> Result<AppServerClient<InProcessTransport>, ClientError> {
    let mut client = AppServerClient::new(transport);
    let initialized = client.initialize(InitializeParams {
        client_info,
        capabilities,
    })?;
    let expected_schema = schema_hash();
    if initialized.schema_hash.0 != expected_schema {
        return Err(ClientError::Protocol(format!(
            "schema hash mismatch: client expected {expected_schema}, server returned {}",
            initialized.schema_hash.0
        )));
    }
    Ok(client)
}
