use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use crate::features::config::preferred_model;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::HookEnablementDto;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::config::PluginRequestEnablementDto;
use zeta_app_server_protocol::protocol::config::SkillSourceEnablementDto;

pub(crate) fn config_view(config: &ConfigReadResult) -> PaneViewModel<SelectionViewModel> {
    PaneViewModel::new(
        SelectionViewModel::new(
            "Config",
            vec![
                SelectionTab::new("Overview", overview(config)),
                SelectionTab::new("Providers", providers(config)),
                SelectionTab::new("MCP", mcp_servers(config)),
                SelectionTab::new("Skill sources", skill_sources(config)),
                SelectionTab::new("Plugins", plugins(config)),
                SelectionTab::new("Hooks", hooks(config)),
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
        detail("Preferred model", preferred_model(config)),
        detail(
            "Approval review model",
            approval_review_model(&config.approval_review_model),
        ),
        detail("Providers", config.providers.len().to_string()),
        detail("MCP servers", config.mcp_servers.len().to_string()),
        detail("Skill sources", config.skill_sources.len().to_string()),
        detail("Plugins", config.plugin_requests.len().to_string()),
        detail("Hooks", config.hooks.len().to_string()),
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

fn mcp_servers(config: &ConfigReadResult) -> Vec<SelectionItem> {
    or_empty(
        config
            .mcp_servers
            .values()
            .map(|server| {
                let state = match server.enablement {
                    McpServerEnablementDto::Disabled => "disabled",
                    McpServerEnablementDto::Enabled => "enabled",
                };
                detail(&server.display_name, format!("{}  ·  {state}", server.id))
            })
            .collect(),
        "No MCP servers configured",
    )
}

fn skill_sources(config: &ConfigReadResult) -> Vec<SelectionItem> {
    or_empty(
        config
            .skill_sources
            .values()
            .map(|source| {
                let state = match source.enablement {
                    SkillSourceEnablementDto::Disabled => "disabled",
                    SkillSourceEnablementDto::Enabled => "enabled",
                };
                detail(&source.id, format!("{state}  ·  {}", source.root_reference))
            })
            .collect(),
        "No skill sources configured",
    )
}

fn plugins(config: &ConfigReadResult) -> Vec<SelectionItem> {
    or_empty(
        config
            .plugin_requests
            .values()
            .map(|plugin| {
                let state = match plugin.enablement {
                    PluginRequestEnablementDto::Disabled => "disabled",
                    PluginRequestEnablementDto::Enabled => "enabled",
                };
                detail(&plugin.plugin_id, format!("{}  ·  {state}", plugin.version))
            })
            .collect(),
        "No plugins configured",
    )
}

fn hooks(config: &ConfigReadResult) -> Vec<SelectionItem> {
    or_empty(
        config
            .hooks
            .values()
            .map(|hook| {
                let state = match hook.enablement {
                    HookEnablementDto::Disabled => "disabled",
                    HookEnablementDto::Enabled => "enabled",
                };
                detail(
                    &hook.id,
                    format!("{:?}  ·  {state}", hook.event).to_lowercase(),
                )
            })
            .collect(),
        "No hooks configured",
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
