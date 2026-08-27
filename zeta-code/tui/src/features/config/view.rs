use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use crate::features::config::preferred_model;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;

pub(crate) fn config_view(config: &ConfigReadResult) -> PaneViewModel<SelectionViewModel> {
    PaneViewModel::new(
        SelectionViewModel::new(
            "Config",
            vec![
                SelectionTab::new("Overview", overview(config)),
                SelectionTab::new("Providers", providers(config)),
                SelectionTab::new("Language servers", language_servers(config)),
            ],
        )
        .with_search(SearchBoxModel::new("Search configuration"))
        .with_empty_message("No matching configuration"),
        "Space search  ·  ←/→ tabs  ·  ↑/↓ inspect  ·  Esc back",
    )
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
