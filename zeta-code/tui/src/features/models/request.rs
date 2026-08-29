use super::ModelPaneSpec;
use super::model_pane_spec;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
) -> Result<ModelPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    let catalog = client.list_models()?;
    Ok(model_pane_spec(&catalog, config.preferred_model.as_ref()))
}
