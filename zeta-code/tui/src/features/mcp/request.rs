use super::McpSelectionView;
use super::mcp_selection_view;
use crate::client::new_command_id;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::config::McpServerSetEnablementParams;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
) -> Result<McpSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_config()
        .map(|config| mcp_selection_view(&config))
}

pub(crate) fn set_enablement<T>(
    client: &mut AppServerClient<T>,
    server_id: String,
    enablement: McpServerEnablementDto,
) -> Result<McpSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    client.set_mcp_server_enablement(McpServerSetEnablementParams {
        command_id: new_command_id("mcp-enablement"),
        expected_revision: config.revision,
        server_id,
        enablement,
    })?;
    load_selection(client)
}
