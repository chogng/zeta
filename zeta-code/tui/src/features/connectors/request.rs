use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::connectors::ConnectorDisconnectParams;

use super::ConnectorSelectionView;
use super::connector_selection_view;
use crate::client::new_command_id;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
) -> Result<ConnectorSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_connectors()
        .map(|catalog| connector_selection_view(&catalog))
}

pub(crate) fn disconnect<T>(
    client: &mut AppServerClient<T>,
    connector_id: String,
) -> Result<ConnectorSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    let catalog = client.list_connectors()?;
    client.disconnect_connector(ConnectorDisconnectParams {
        command_id: new_command_id("connector-disconnect").to_string(),
        expected_generation: catalog.generation,
        connector_id,
    })?;
    load_selection(client)
}
