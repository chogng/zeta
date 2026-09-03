use super::ConfigChoices;
use super::ConfigEdit;
use super::ConfigEditResult;
use super::ProviderApiKeyEdit;
use super::TerminalSettings;
use super::config_choices;
use crate::client::new_command_id;
use std::fmt;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::ProviderApiKeySetRequest;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
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
    let providers = client.list_providers()?;
    Ok(config_choices(&server_config, &providers, terminal))
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

pub(crate) fn set_terminal_settings(
    client: &mut AppServerRequestHandle,
    edit: ConfigEdit,
) -> Result<ConfigEditResult, ConfigCommandError> {
    let tui = edit
        .terminal
        .write_to_tui(&edit.server_config.tui)
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
        choices: config_choices(&config, &edit.providers, settings),
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
