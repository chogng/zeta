//! MCP stdio adapter that exposes Zeta's canonical App Server Agent execution path.

mod agent;
mod events;
mod http;
mod interaction;
mod options;
mod protocol;
mod receipt;
mod server;

use agent::AppServerAgentService;
use receipt::ReceiptStore;
use std::sync::Arc;
use zeta_app_server_client::{
    InProcessClientOptions, open_in_process_app_server, start_in_process_client,
};
use zeta_app_server_protocol::protocol::common::ClientInfo;

pub use options::{HttpServerOptions, McpServerOptions};
pub use server::McpServerError;

/// Runs one MCP stdio connection backed by an embedded App Server.
///
/// The server owns only MCP framing and projection. Session, Thread, Turn, model, Tool, policy,
/// and durable state remain owned by the App Server composition root.
pub fn run_stdio(options: McpServerOptions) -> Result<(), McpServerError> {
    options.validate().map_err(McpServerError::configuration)?;
    let receipts = open_receipts(options.profile_root())?;
    let client = start_in_process_client(
        InProcessClientOptions::new(
            options.profile_root(),
            ClientInfo {
                name: "zeta-mcp-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        )
        .with_dir_root(options.dir_root()),
    )
    .map_err(McpServerError::app_server)?;
    let agent = Arc::new(AppServerAgentService::with_receipts(
        client,
        options.runtime_limits(),
        receipts,
        "stdio:local-user".into(),
    ));
    server::serve_stdio(agent)
}

/// Runs an authenticated MCP Streamable HTTP endpoint backed by one shared embedded App Server.
pub fn run_http(
    options: McpServerOptions,
    http_options: HttpServerOptions,
) -> Result<(), McpServerError> {
    options.validate().map_err(McpServerError::configuration)?;
    http_options
        .validate()
        .map_err(McpServerError::configuration)?;
    let receipts = open_receipts(options.profile_root())?;
    let host = open_in_process_app_server(
        InProcessClientOptions::new(
            options.profile_root(),
            ClientInfo {
                name: "zeta-mcp-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        )
        .with_dir_root(options.dir_root()),
    )
    .map_err(McpServerError::app_server)?;
    http::serve(host, options.runtime_limits(), receipts, http_options)
}

fn open_receipts(profile_root: &std::path::Path) -> Result<Arc<ReceiptStore>, McpServerError> {
    let state = zeta_state::StateRuntime::open(profile_root).map_err(McpServerError::receipt)?;
    ReceiptStore::open(state.database_path())
        .map(Arc::new)
        .map_err(McpServerError::receipt)
}
