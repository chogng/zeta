use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use crate::features::config::TerminalSettings;
use crate::features::config::TerminalSettingsEdit;
use crate::features::config::preferred_model;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigEdit {
    pub(crate) terminal: TerminalSettingsEdit,
    pub(crate) server_config: ConfigReadResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConfigSelectionAction {
    SetMouseInteractions(ConfigEdit),
}

pub(crate) struct ConfigSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, ConfigSelectionAction>,
}

pub(crate) fn config_view(
    config: &ConfigReadResult,
    terminal: TerminalSettings,
    terminal_revision: u64,
) -> ConfigSelectionView {
    let mut actions = BTreeMap::new();
    let mouse_id = SelectionItemId::new("terminal-mouse-interactions");
    let mouse_enabled = terminal.mouse_interactions();
    actions.insert(
        mouse_id.clone(),
        ConfigSelectionAction::SetMouseInteractions(ConfigEdit {
            terminal: TerminalSettingsEdit {
                expected_revision: terminal_revision,
                mouse_interactions: !mouse_enabled,
            },
            server_config: config.clone(),
        }),
    );
    let enhanced_terminal = vec![
        SelectionItem::new("Mouse interactions")
            .with_id(mouse_id)
            .with_columns(
                "Mouse interactions",
                "Clicks and hover in interactive panes",
                mouse_enabled.to_string(),
            ),
    ];

    ConfigSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Config",
                vec![
                    SelectionTab::new("Overview", overview(config)),
                    SelectionTab::new("Enhanced terminal", enhanced_terminal),
                    SelectionTab::new("Providers", providers(config)),
                    SelectionTab::new("Language servers", language_servers(config)),
                ],
            )
            .with_search(SearchBoxModel::new("Search configuration"))
            .with_empty_message("No matching configuration"),
            "Space search  ·  Enter toggle  ·  ←/→ tabs  ·  ↑/↓ inspect  ·  Esc back",
        ),
        actions,
    }
}

fn overview(config: &ConfigReadResult) -> Vec<SelectionItem> {
    vec![
        detail("Revision", config.revision.to_string()),
        detail("Generation", config.generation.to_string()),
        detail(
            "Preferred model",
            preferred_model(config.preferred_model.as_ref()),
        ),
        detail(
            "Approval review model",
            approval_review_model(&config.approval_review_model),
        ),
        detail("Providers", config.providers.len().to_string()),
        detail(
            "Language servers",
            config.language_servers.len().to_string(),
        ),
    ]
}

fn providers(config: &ConfigReadResult) -> Vec<SelectionItem> {
    or_empty(
        config
            .providers
            .values()
            .map(|provider| {
                let base_url = provider.base_url.as_deref().unwrap_or("default endpoint");
                let tokens = provider
                    .max_output_tokens
                    .map(|value| format!("{value} max output tokens"))
                    .unwrap_or_else(|| "default output limit".into());
                detail(&provider.provider, format!("{base_url}  ·  {tokens}"))
            })
            .collect(),
        "No providers configured",
    )
}

fn language_servers(config: &ConfigReadResult) -> Vec<SelectionItem> {
    or_empty(
        config
            .language_servers
            .iter()
            .map(|(language, server)| {
                let mode = match server.mode {
                    LanguageServerModeDto::Disabled => "disabled",
                    LanguageServerModeDto::Automatic => "automatic",
                    LanguageServerModeDto::Enabled => "enabled",
                };
                detail(
                    language,
                    server
                        .executable
                        .as_deref()
                        .map(|executable| format!("{mode}  ·  {executable}"))
                        .unwrap_or_else(|| mode.into()),
                )
            })
            .collect(),
        "No language servers configured",
    )
}

fn approval_review_model(selection: &ApprovalReviewModelSelectionDto) -> String {
    match selection {
        ApprovalReviewModelSelectionDto::Automatic => "automatic".into(),
        ApprovalReviewModelSelectionDto::Explicit { model } => {
            format!("{}/{}", model.provider, model.model)
        }
    }
}

fn detail(label: &str, value: impl Into<String>) -> SelectionItem {
    SelectionItem::new(label).with_description(value)
}

fn or_empty(items: Vec<SelectionItem>, message: &str) -> Vec<SelectionItem> {
    if items.is_empty() {
        vec![SelectionItem::new(message)]
    } else {
        items
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
