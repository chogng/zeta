use super::ModelSelectionView;
use super::model_selection_view;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
) -> Result<ModelSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    let catalog = client.list_models()?;
    Ok(model_selection_view(&catalog, &config))
}
