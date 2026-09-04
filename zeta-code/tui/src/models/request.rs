use super::Command;
use super::ModelChoices;
use super::ModelSummary;
use super::model_choices;
use crate::client::new_command_id;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_protocol::Patch;

pub(crate) struct PreferredModelUpdate {
    pub(crate) summary: ModelSummary,
    pub(crate) notice: String,
}

impl Command {
    pub(crate) const fn request_name(&self) -> &'static str {
        match self {
            Self::SetPreferred { .. } => "zeta-tui-set-preferred-model",
        }
    }

    pub(crate) fn command_line(&self) -> String {
        match self {
            Self::SetPreferred { preference } => format!("/model {preference}"),
        }
    }
}

pub(crate) fn execute<T>(
    client: &mut AppServerClient<T>,
    command: Command,
) -> Result<PreferredModelUpdate, String>
where
    T: JsonRpcTransport,
{
    match command {
        Command::SetPreferred { preference } => set_preferred_model(client, &preference),
    }
    .map_err(|error| error.to_string())
}

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
) -> Result<ModelChoices, ClientError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    let catalog = client.list_models()?;
    Ok(model_choices(&catalog, config.preferred_model.as_ref()))
}

pub(crate) fn set_preferred_model<T>(
    client: &mut AppServerClient<T>,
    arguments: &str,
) -> Result<PreferredModelUpdate, ModelCommandError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    if arguments.is_empty() {
        return Err(ModelCommandError(
            "model selection requires a model or 'clear'".into(),
        ));
    }

    let preferred_model = if arguments == "clear" {
        Patch::Null
    } else {
        let (provider, model) = arguments.split_once('/').ok_or_else(|| {
            ModelCommandError(
                "model must use <provider>/<model>; use /model clear to unset it".into(),
            )
        })?;
        if provider.trim().is_empty()
            || model.trim().is_empty()
            || provider.contains(char::is_whitespace)
            || model.contains(char::is_whitespace)
        {
            return Err(ModelCommandError(
                "model must use non-empty <provider>/<model> without whitespace".into(),
            ));
        }
        if !config.providers.contains_key(provider) {
            return Err(ModelCommandError(format!(
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
        preferred_model,
        commit_message_model: Patch::Missing,
        approval_review_model: Patch::Missing,
        tool_mode: Patch::Missing,
        agent_grep_backend: Patch::Missing,
        gui: Patch::Missing,
        tui: Patch::Missing,
    })?;
    let config = client.read_config()?;
    let summary = ModelSummary::from_catalog(config.preferred_model, None);
    let notice = format!(
        "Preferred model: {}",
        preferred_model_label(summary.preferred_model())
    );
    Ok(PreferredModelUpdate { summary, notice })
}

pub(crate) fn preferred_model_label(model: Option<&ModelRefDto>) -> String {
    model
        .map(|model| format!("{}/{}", model.provider, model.model))
        .unwrap_or_else(|| "not configured".into())
}

#[derive(Debug)]
pub(crate) struct ModelCommandError(String);

impl fmt::Display for ModelCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<ClientError> for ModelCommandError {
    fn from(error: ClientError) -> Self {
        Self(error.to_string())
    }
}
