use crate::client::new_command_id;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::config::CodeProductPreferencesUpdateDto;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::config::ProductsConfigUpdateDto;
use zeta_protocol::Patch;

pub(crate) struct PreferredModelUpdate {
    pub(crate) preferred_model: Option<ModelRefDto>,
    pub(crate) notice: String,
}

pub(crate) fn set_preferred_model<T>(
    client: &mut AppServerClient<T>,
    arguments: &str,
) -> Result<PreferredModelUpdate, ConfigCommandError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    if arguments.is_empty() {
        return Err(ConfigCommandError(
            "model selection requires a model or 'clear'".into(),
        ));
    }

    let preferred_model_patch = if arguments == "clear" {
        Patch::Null
    } else {
        let (provider, model) = arguments.split_once('/').ok_or_else(|| {
            ConfigCommandError(
                "model must use <provider>/<model>; use /model clear to unset it".into(),
            )
        })?;
        if provider.trim().is_empty()
            || model.trim().is_empty()
            || provider.contains(char::is_whitespace)
            || model.contains(char::is_whitespace)
        {
            return Err(ConfigCommandError(
                "model must use non-empty <provider>/<model> without whitespace".into(),
            ));
        }
        if !config.providers.contains_key(provider) {
            return Err(ConfigCommandError(format!(
                "provider '{provider}' is not configured"
            )));
        }
        Patch::Value(ModelRefDto {
            provider: provider.into(),
            model: model.into(),
        })
    };

    client.update_config(ConfigUpdateParams {
        command_id: new_command_id("model"),
        expected_revision: config.revision,
        preferred_model: preferred_model_patch,
        approval_review_model: Patch::Missing,
        products: None,
    })?;
    let config = client.read_config()?;
    let notice = format!(
        "Preferred model: {}",
        preferred_model(config.preferred_model.as_ref())
    );
    Ok(PreferredModelUpdate {
        preferred_model: config.preferred_model,
        notice,
    })
}

pub(crate) fn set_code_theme<T>(
    client: &mut AppServerClient<T>,
    preference: &str,
) -> Result<(), ConfigCommandError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    client.update_config(ConfigUpdateParams {
        command_id: new_command_id("theme"),
        expected_revision: config.revision,
        preferred_model: Patch::Missing,
        approval_review_model: Patch::Missing,
        products: Some(ProductsConfigUpdateDto {
            code: Some(CodeProductPreferencesUpdateDto {
                color_theme: Patch::Value(preference.to_owned()),
            }),
            ..Default::default()
        }),
    })?;
    Ok(())
}

pub(crate) fn preferred_model(model: Option<&ModelRefDto>) -> String {
    model
        .map(|model| format!("{}/{}", model.provider, model.model))
        .unwrap_or_else(|| "not configured".into())
}

#[derive(Debug)]
pub(crate) struct ConfigCommandError(String);

impl fmt::Display for ConfigCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<ClientError> for ConfigCommandError {
    fn from(error: ClientError) -> Self {
        Self(error.to_string())
    }
}
