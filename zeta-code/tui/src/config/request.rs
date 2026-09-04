use super::ConfigChoices;
use super::ConfigEdit;
use super::ConfigEditResult;
use super::LanguageServerEdit;
use super::ProviderApiKeyEdit;
use super::TerminalSettings;
use super::config_choices;
use crate::client::new_command_id;
use crate::status::StatusLineSettings;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_client::ProviderApiKeySetRequest;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigureParams;
use zeta_protocol::Patch;

pub(crate) struct ProviderApiKeyUpdate {
    pub(crate) provider: String,
    pub(crate) choices: ConfigChoices,
}

pub(crate) fn read_config_choices(
    client: &mut AppServerRequestHandle,
) -> Result<ConfigChoices, ConfigCommandError> {
    let server_config = client.read_config()?;
    let terminal = TerminalSettings::from_tui(&server_config.tui).map_err(ConfigCommandError)?;
    let status_line =
        StatusLineSettings::from_tui(&server_config.tui).map_err(ConfigCommandError)?;
    let providers = client.list_providers()?;
    Ok(config_choices(
        &server_config,
        &providers,
        terminal,
        status_line,
    ))
}

pub(crate) fn set_provider_api_key(
    client: &mut AppServerRequestHandle,
    edit: ProviderApiKeyEdit,
) -> Result<ProviderApiKeyUpdate, ConfigCommandError> {
    let (provider, api_key) = edit.into_parts();
    client.set_provider_api_key(ProviderApiKeySetRequest::new(provider.clone(), api_key))?;
    let choices = read_config_choices(client)?;
    Ok(ProviderApiKeyUpdate { provider, choices })
}

pub(crate) fn set_settings<T>(
    client: &mut AppServerClient<T>,
    edit: ConfigEdit,
) -> Result<ConfigEditResult, ConfigCommandError>
where
    T: JsonRpcTransport,
{
    let tui = edit
        .terminal
        .write_to_tui(&edit.server_config.tui)
        .map_err(ConfigCommandError)?;
    let tui = edit.status_line.write_to_tui(&tui);
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
    let terminal = TerminalSettings::from_tui(&config.tui).map_err(ConfigCommandError)?;
    let status_line = StatusLineSettings::from_tui(&config.tui).map_err(ConfigCommandError)?;
    Ok(ConfigEditResult {
        terminal,
        status_line: status_line.clone(),
        choices: config_choices(&config, &edit.providers, terminal, status_line),
    })
}

pub(crate) fn set_language_server_mode<T>(
    client: &mut AppServerClient<T>,
    edit: LanguageServerEdit,
) -> Result<ConfigEditResult, ConfigCommandError>
where
    T: JsonRpcTransport,
{
    client.configure_language_server(LanguageServerConfigureParams {
        command_id: new_command_id("language-server"),
        expected_revision: edit.expected_revision,
        server_id: edit.server_id,
        config: edit.config,
    })?;
    let config = client.read_config()?;
    let terminal = TerminalSettings::from_tui(&config.tui).map_err(ConfigCommandError)?;
    let status_line = StatusLineSettings::from_tui(&config.tui).map_err(ConfigCommandError)?;
    let providers = client.list_providers()?;
    Ok(ConfigEditResult {
        terminal,
        status_line: status_line.clone(),
        choices: config_choices(&config, &providers, terminal, status_line),
    })
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

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
