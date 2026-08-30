use super::ConfigPaneSpec;
use super::PermissionEdit;
use super::ProviderApiKeyEdit;
use super::TerminalSettingsSnapshot;
use super::config_pane_spec;
use crate::client::new_command_id;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_client::ProviderApiKeySetRequest;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListParams;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;
use zeta_protocol::Patch;
use zeta_protocol::SessionId;

pub(crate) struct ProviderApiKeyUpdate {
    pub(crate) provider: String,
    pub(crate) pane_spec: ConfigPaneSpec,
}

pub(crate) fn read_config_pane(
    client: &mut AppServerRequestHandle,
    session_id: &SessionId,
    terminal: TerminalSettingsSnapshot,
) -> Result<ConfigPaneSpec, ClientError> {
    let server_config = client.read_config()?;
    let providers = client.list_providers()?;
    let dirs = client.list_session_dirs(SessionDirListParams {
        session_id: session_id.clone(),
    })?;
    Ok(config_pane_spec(
        &server_config,
        &providers,
        terminal.settings,
        terminal.revision,
        session_id,
        &dirs,
    ))
}

pub(crate) fn set_provider_api_key(
    client: &mut AppServerRequestHandle,
    edit: ProviderApiKeyEdit,
    terminal: TerminalSettingsSnapshot,
    session_id: &SessionId,
) -> Result<ProviderApiKeyUpdate, ClientError> {
    let (provider, api_key) = edit.into_parts();
    client.set_provider_api_key(ProviderApiKeySetRequest::new(provider.clone(), api_key))?;
    let pane_spec = read_config_pane(client, session_id, terminal)?;
    Ok(ProviderApiKeyUpdate {
        provider,
        pane_spec,
    })
}

pub(crate) fn set_permissions(
    client: &mut AppServerRequestHandle,
    edit: PermissionEdit,
) -> Result<ConfigPaneSpec, ClientError> {
    let result = client.set_session_dir_permissions(edit.params.clone())?;
    Ok(config_pane_spec(
        &edit.server_config,
        &edit.providers,
        edit.terminal,
        edit.terminal_revision,
        &edit.params.session_id,
        &SessionDirListResult {
            revision: result.revision,
            dirs: result.dirs,
        },
    ))
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
        tui_theme: Patch::Missing,
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
    client.update_config(ConfigUpdateParams {
        command_id: new_command_id("theme"),
        expected_revision: config.revision,
        preferred_model: Patch::Missing,
        approval_review_model: Patch::Missing,
        commit_message_model: Patch::Missing,
        tool_mode: Patch::Missing,
        agent_grep_backend: Patch::Missing,
        tui_theme: Patch::Value(theme),
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
