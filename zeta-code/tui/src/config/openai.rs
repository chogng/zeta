use super::ConfigChoices;
use super::ConfigSelectionAction;
use crate::client::new_command_id;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::ProviderConfigDto;
use zeta_app_server_protocol::protocol::config::ProviderConfigureParams;
use zeta_app_server_protocol::protocol::provider::ProviderListResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Settings {
    revision: u64,
    custom: ProviderConfigDto,
    api_address: String,
    api_key_saved: bool,
    custom_key_saved: bool,
}

impl Settings {
    pub(crate) fn new(config: &ConfigReadResult, providers: &ProviderListResult) -> Self {
        let saved = |id| {
            providers
                .providers
                .iter()
                .any(|entry| entry.provider == id && entry.api_key_configured)
        };
        Self {
            revision: config.revision,
            custom: config
                .providers
                .get("openai-compatible")
                .cloned()
                .unwrap_or_else(|| ProviderConfigDto {
                    provider: "openai-compatible".into(),
                    base_url: None,
                    max_output_tokens: None,
                    model_context: BTreeMap::new(),
                }),
            api_address: config
                .providers
                .get("openai")
                .and_then(|entry| entry.base_url.clone())
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            api_key_saved: saved("openai"),
            custom_key_saved: saved("openai-compatible"),
        }
    }
}

pub(crate) fn choices(settings: &Settings) -> ConfigChoices {
    let mut actions = BTreeMap::new();
    let mut item = |id: &str, label: &str, description: String, action| {
        let id = ListSelectionItemId::new(id);
        actions.insert(id.clone(), action);
        ListSelectionItem::new(label)
            .with_id(id)
            .with_description(description)
    };
    let items =
        vec![
            item(
                "api-key",
                "OpenAI API key",
                format!(
                    "{} · {}",
                    key_status(settings.api_key_saved),
                    settings.api_address
                ),
                ConfigSelectionAction::OpenProviderApiKey {
                    provider: "openai".into(),
                    display_name: "OpenAI".into(),
                },
            ),
            item(
                "custom-address",
                "Custom base URL",
                settings.custom.base_url.clone().unwrap_or_else(|| {
                    "Not configured · OpenAI-compatible Chat Completions".into()
                }),
                ConfigSelectionAction::OpenEndpoint(ProviderConfigureParams {
                    command_id: new_command_id("provider"),
                    expected_revision: settings.revision,
                    config: settings.custom.clone(),
                }),
            ),
            item(
                "custom-key",
                "Custom API key",
                format!(
                    "{} · Separate from your OpenAI API key",
                    key_status(settings.custom_key_saved)
                ),
                ConfigSelectionAction::OpenProviderApiKey {
                    provider: "openai-compatible".into(),
                    display_name: "Custom service".into(),
                },
            ),
            item(
                "chatgpt",
                "ChatGPT subscription",
                "Sign in or manage your ChatGPT account".into(),
                ConfigSelectionAction::OpenSubscription,
            ),
        ];
    ConfigChoices {
        model: ListSelectionModel::new(
            "OpenAI",
            vec![ListSelectionGroup::new("Connections", items)],
        ),
        actions,
    }
}

fn key_status(saved: bool) -> &'static str {
    if saved {
        "Key saved · Not verified"
    } else {
        "No saved key"
    }
}
