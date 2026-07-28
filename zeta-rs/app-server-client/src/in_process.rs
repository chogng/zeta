use crate::AppServerClient;
use crate::ClientError;
use crate::JsonRpcTransport;
use std::path::PathBuf;
use zeta_app_server::AppServer;
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
    pub client_info: ClientInfo,
    pub capabilities: ClientCapabilities,
    pub slash_commands: SlashCommandCatalog,
}

impl InProcessClientOptions {
    pub fn new(state_root: impl Into<PathBuf>, client_info: ClientInfo) -> Self {
        Self {
            state_root: state_root.into(),
            client_info,
            capabilities: ClientCapabilities::default(),
            slash_commands: SlashCommandCatalog::default(),
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
}

/// In-memory transport that still exercises the versioned JSON-RPC dispatcher.
pub struct InProcessTransport {
    server: AppServer,
    connection: ConnectionState,
    notifications: Vec<String>,
}

impl InProcessTransport {
    /// Creates an embedded transport that routes every request through the App Server dispatcher.
    ///
    /// Hosts that provide their own composition root can use this instead of the local filesystem
    /// composition used by [`start_in_process_client`].
    pub fn from_server(server: AppServer) -> Self {
        let connection = server.connection();
        Self {
            server,
            connection,
            notifications: Vec::new(),
        }
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
        Ok(std::mem::take(&mut self.notifications))
    }
}

/// Opens, initializes, and schema-checks an embedded App Server client.
pub fn start_in_process_client(
    options: InProcessClientOptions,
) -> Result<AppServerClient<InProcessTransport>, ClientError> {
    let server = open_local_app_server(
        LocalAppServerOptions::new(options.state_root)
            .with_slash_command_catalog(options.slash_commands),
    )
    .map_err(|error| ClientError::Transport(error.to_string()))?;
    let mut client = AppServerClient::new(InProcessTransport::from_server(server));
    let initialized = client.initialize(InitializeParams {
        client_info: options.client_info,
        capabilities: options.capabilities,
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
