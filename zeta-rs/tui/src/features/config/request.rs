use crate::client::new_command_id;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_protocol::Patch;

pub(crate) enum PreferredModelOutcome {
    Shown(String),
    Updated {
        config: ConfigReadResult,
        notice: String,
    },
}

pub(crate) fn config_summary<T>(client: &mut AppServerClient<T>) -> Result<String, ClientError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    Ok(format!(
        "Config revision: {}\nModel: {}\nProviders: {}\nMCP servers: {}\nSkill sources: {}",
        config.revision,
        preferred_model(&config),
        config.providers.len(),
        config.mcp_servers.len(),
        config.skill_sources.len()
    ))
}

pub(crate) fn mcp_summary<T>(client: &mut AppServerClient<T>) -> Result<String, ClientError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    if config.mcp_servers.is_empty() {
        return Ok("No MCP servers configured.".into());
    }
    Ok(config
        .mcp_servers
        .values()
        .map(|server| {
            let state = match server.enablement {
                McpServerEnablementDto::Disabled => "disabled",
                McpServerEnablementDto::Enabled => "enabled",
            };
            format!("{}  {}  {state}", server.id, server.display_name)
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(crate) fn set_or_show_preferred_model<T>(
    client: &mut AppServerClient<T>,
    arguments: &str,
) -> Result<PreferredModelOutcome, ConfigCommandError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    if arguments.is_empty() {
        return Ok(PreferredModelOutcome::Shown(format!(
            "Preferred model: {}",
            preferred_model(&config)
        )));
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
    })?;
    let config = client.read_config()?;
    let notice = format!("Preferred model: {}", preferred_model(&config));
    Ok(PreferredModelOutcome::Updated { config, notice })
}

pub(crate) fn preferred_model(config: &ConfigReadResult) -> String {
    config
        .preferred_model
        .as_ref()
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
