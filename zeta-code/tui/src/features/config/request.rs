use super::ConfigEdit;
use super::ConfigEditResult;
use super::ConfigChoices;
use super::PermissionEdit;
use super::ProviderApiKeyEdit;
use super::TerminalSettings;
use super::config_choices;
use crate::client::new_command_id;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_client::ProviderApiKeySetRequest;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListParams;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;
use zeta_protocol::Patch;
use zeta_protocol::SessionId;

pub(crate) struct ProviderApiKeyUpdate {
    pub(crate) provider: String,
    pub(crate) region_spec: ConfigChoices,
}

pub(crate) fn read_config_region(
    client: &mut AppServerRequestHandle,
    session_id: &SessionId,
) -> Result<ConfigChoices, ConfigCommandError> {
    let server_config = client.read_config()?;
    let terminal = TerminalSettings::from_tui(&server_config.tui).map_err(ConfigCommandError)?;
    let providers = client.list_providers()?;
    let dirs = client.list_session_dirs(SessionDirListParams {
        session_id: session_id.clone(),
    })?;
    Ok(config_choices(
        &server_config,
        &providers,
        terminal,
        session_id,
        &dirs,
    ))
}

pub(crate) fn set_provider_api_key(
    client: &mut AppServerRequestHandle,
    edit: ProviderApiKeyEdit,
    session_id: &SessionId,
) -> Result<ProviderApiKeyUpdate, ConfigCommandError> {
    let (provider, api_key) = edit.into_parts();
    client.set_provider_api_key(ProviderApiKeySetRequest::new(provider.clone(), api_key))?;
    let region_spec = read_config_region(client, session_id)?;
    Ok(ProviderApiKeyUpdate {
        provider,
        region_spec,
    })
}

pub(crate) fn set_permissions(
    client: &mut AppServerRequestHandle,
    edit: PermissionEdit,
) -> Result<ConfigChoices, ConfigCommandError> {
    let result = client.set_session_dir_permissions(edit.params.clone())?;
    Ok(config_choices(
        &edit.server_config,
        &edit.providers,
        edit.terminal,
        &edit.params.session_id,
        &SessionDirListResult {
            revision: result.revision,
            dirs: result.dirs,
        },
    ))
}

pub(crate) fn set_terminal_settings(
    client: &mut AppServerRequestHandle,
    edit: ConfigEdit,
) -> Result<ConfigEditResult, ConfigCommandError> {
    let tui = edit
        .terminal
        .validate()
        .and_then(|settings| settings.write_to_tui(&edit.server_config.tui))
        .map_err(ConfigCommandError)?;
    client.update_config(ConfigUpdateParams {
        command_id: new_command_id("tui"),
        expected_revision: edit.server_config.revision,
        preferred_model: Patch::Missing,
        approval_review_model: Patch::Missing,
        commit_message_model: Patch::Missing,
        tool_mode: Patch::Missing,
        agent_grep_backend: Patch::Missing,
        gui: Patch::Missing,
        tui: Patch::Value(tui),
    })?;
    let config = client.read_config()?;
    let settings = TerminalSettings::from_tui(&config.tui).map_err(ConfigCommandError)?;
    Ok(ConfigEditResult {
        settings,
        region_spec: config_choices(
            &config,
            &edit.providers,
            settings,
            &edit.session_id,
            &edit.dirs,
        ),
    })
}

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
        commit_message_model: Patch::Missing,
        approval_review_model: Patch::Missing,
        tool_mode: Patch::Missing,
        agent_grep_backend: Patch::Missing,
        gui: Patch::Missing,
        tui: Patch::Missing,
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

pub(crate) fn set_tui_theme<T>(
    client: &mut AppServerClient<T>,
    theme: String,
) -> Result<(), ConfigCommandError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    let mut tui = config.tui.0;
    tui.insert("theme".into(), serde_json::Value::String(theme));
    client.update_config(ConfigUpdateParams {
        command_id: new_command_id("theme"),
        expected_revision: config.revision,
        preferred_model: Patch::Missing,
        approval_review_model: Patch::Missing,
        commit_message_model: Patch::Missing,
        tool_mode: Patch::Missing,
        agent_grep_backend: Patch::Missing,
        gui: Patch::Missing,
        tui: Patch::Value(FrontendConfigDto(tui)),
    })?;
    Ok(())
}

pub(crate) fn tui_theme(config: &ConfigReadResult) -> &str {
    config
        .tui
        .0
        .get("theme")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("system")
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
