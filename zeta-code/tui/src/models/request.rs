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
    pub(crate) picker: Option<ModelChoices>,
}

impl Command {
    pub(crate) const fn request_name(&self) -> &'static str {
        match self {
            Self::Pin { .. } => "zeta-tui-pin-model",
            Self::SetPreferred { .. } => "zeta-tui-set-preferred-model",
        }
    }

    pub(crate) fn command_line(&self) -> String {
        match self {
            Self::Pin { preference, pinned } => format!(
                "/model {} {preference}",
                if *pinned { "pin" } else { "unpin" }
            ),
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
        Command::Pin { preference, pinned } => set_pin(client, &preference, pinned),
    }
    .map_err(|error| error.to_string())
}

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
) -> Result<ModelChoices, ModelCommandError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    let catalog = client.list_models()?;
    let providers = client.list_providers()?;
    model_choices(&catalog, &config, &providers).map_err(ModelCommandError)
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
        features: Default::default(),
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
    Ok(PreferredModelUpdate {
        summary,
        notice,
        picker: None,
    })
}

fn preferred_model_label(model: Option<&ModelRefDto>) -> String {
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

fn write_pins<T: JsonRpcTransport>(
    client: &mut AppServerClient<T>,
    config: zeta_app_server_protocol::protocol::config::ConfigReadResult,
    pins: Vec<ModelRefDto>,
) -> Result<(), ModelCommandError> {
    let mut tui = config.tui;
    tui.0.insert(
        "pinnedModels".into(),
        serde_json::to_value(pins).map_err(|error| ModelCommandError(error.to_string()))?,
    );
    client.update_config(ConfigUpdateParams {
        features: Default::default(),
        command_id: new_command_id("pin-model"),
        expected_revision: config.revision,
        preferred_model: Patch::Missing,
        commit_message_model: Patch::Missing,
        approval_review_model: Patch::Missing,
        tool_mode: Patch::Missing,
        agent_grep_backend: Patch::Missing,
        gui: Patch::Missing,
        tui: Patch::Value(tui),
    })?;
    Ok(())
}
fn set_pin<T: JsonRpcTransport>(
    client: &mut AppServerClient<T>,
    preference: &str,
    pinned: bool,
) -> Result<PreferredModelUpdate, ModelCommandError> {
    let config = client.read_config()?;
    let mut pins = super::picker::pinned_models(&config.tui).map_err(ModelCommandError)?;
    let (provider, model) = preference
        .split_once('/')
        .ok_or_else(|| ModelCommandError("Invalid model identity".into()))?;
    let model = ModelRefDto {
        provider: provider.into(),
        model: model.into(),
    };
    if pinned {
        let catalog = client.list_models()?;
        if !catalog.models.iter().any(|entry| {
            entry.model.provider.as_str() == model.provider
                && entry.model.model.as_str() == model.model
        }) {
            return Err(ModelCommandError("Model no longer available".into()));
        }
        if !pins.contains(&model) {
            pins.push(model);
        }
    } else {
        pins.retain(|pin| pin != &model);
    }
    let summary = ModelSummary::from_catalog(config.preferred_model.clone(), None);
    write_pins(client, config, pins)?;
    Ok(PreferredModelUpdate {
        summary,
        notice: if pinned {
            "Model pinned"
        } else {
            "Model unpinned"
        }
        .into(),
        picker: Some(load_selection(client)?),
    })
}
pub(crate) fn remove_provider_pins<T: JsonRpcTransport>(
    client: &mut AppServerClient<T>,
    provider: &str,
) -> Result<(), String> {
    let config = client.read_config().map_err(|error| error.to_string())?;
    let mut pins = super::picker::pinned_models(&config.tui)?;
    let before = pins.len();
    pins.retain(|pin| pin.provider != provider);
    if pins.len() != before {
        write_pins(client, config, pins).map_err(|error| error.to_string())?;
    }
    Ok(())
}
