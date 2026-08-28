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
use std::fmt;
use zeroize::Zeroizing;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionsSetParams;
use zeta_protocol::SessionId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigEdit {
    pub(crate) terminal: TerminalSettingsEdit,
    pub(crate) server_config: ConfigReadResult,
    pub(crate) providers: ProviderListResult,
    pub(crate) session_id: SessionId,
    pub(crate) additional_directories: WorkspaceAdditionalDirectoryListResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AdditionalDirectoryPermissionEdit {
    pub(crate) params: WorkspaceAdditionalDirectoryPermissionsSetParams,
    pub(crate) terminal: TerminalSettings,
    pub(crate) terminal_revision: u64,
    pub(crate) server_config: ConfigReadResult,
    pub(crate) providers: ProviderListResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConfigSelectionAction {
    SetMouseInteractions(ConfigEdit),
    SetAdditionalDirectoryPermissions(AdditionalDirectoryPermissionEdit),
    OpenProviderApiKey {
        provider: String,
        display_name: String,
    },
    SetProviderApiKey {
        provider: String,
    },
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProviderApiKeyEdit {
    provider: String,
    api_key: Zeroizing<String>,
}

impl ProviderApiKeyEdit {
    pub(crate) fn new(provider: String, api_key: String) -> Self {
        Self {
            provider,
            api_key: Zeroizing::new(api_key),
        }
    }

    pub(crate) fn into_parts(mut self) -> (String, String) {
        let provider = std::mem::take(&mut self.provider);
        let api_key = std::mem::take(&mut *self.api_key);
        (provider, api_key)
    }
}

impl fmt::Debug for ProviderApiKeyEdit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderApiKeyEdit")
            .field("provider", &self.provider)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

pub(crate) struct ConfigSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, ConfigSelectionAction>,
}

pub(crate) fn config_view(
    config: &ConfigReadResult,
    providers: &ProviderListResult,
    terminal: TerminalSettings,
    terminal_revision: u64,
    session_id: &SessionId,
    additional_directories: &WorkspaceAdditionalDirectoryListResult,
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
            providers: providers.clone(),
            session_id: session_id.clone(),
            additional_directories: additional_directories.clone(),
        }),
    );
    let mut config_items = vec![
        SelectionItem::new("Mouse interactions")
            .with_id(mouse_id)
            .with_columns(
                "Mouse interactions",
                "Clicks and hover in interactive panes",
                checkbox(mouse_enabled),
            ),
    ];
    config_items.extend(overview(config));
    let provider_items = provider_items(providers, &mut actions);
    let permission_items = additional_directory_permission_items(
        additional_directories,
        session_id,
        terminal,
        terminal_revision,
        config,
        providers,
        &mut actions,
    );

    ConfigSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Config",
                vec![
                    SelectionTab::new("Config", config_items),
                    SelectionTab::new("Directory permissions", permission_items),
                    SelectionTab::new("Providers", provider_items),
                    SelectionTab::new("Language servers", language_servers(config)),
                ],
            )
            .with_search(SearchBoxModel::new("Search configuration"))
            .with_empty_message("No matching configuration"),
            "Space search  ·  Enter select/toggle  ·  ←/→ tabs  ·  ↑/↓ inspect  ·  Esc back",
        ),
        actions,
    }
}

fn additional_directory_permission_items(
    snapshot: &WorkspaceAdditionalDirectoryListResult,
    session_id: &SessionId,
    terminal: TerminalSettings,
    terminal_revision: u64,
    config: &ConfigReadResult,
    providers: &ProviderListResult,
    actions: &mut BTreeMap<SelectionItemId, ConfigSelectionAction>,
) -> Vec<SelectionItem> {
    let mut items = Vec::new();
    for (directory_index, directory) in snapshot.directories.iter().enumerate() {
        for permission in all_additional_directory_permissions() {
            let id = SelectionItemId::new(format!(
                "additional-directory-{directory_index}-{}",
                permission_id(permission)
            ));
            let enabled = directory.permissions.contains(permission);
            actions.insert(
                id.clone(),
                ConfigSelectionAction::SetAdditionalDirectoryPermissions(
                    AdditionalDirectoryPermissionEdit {
                        params: WorkspaceAdditionalDirectoryPermissionsSetParams {
                            session_id: session_id.clone(),
                            root: directory.root.clone(),
                            expected_revision: snapshot.revision,
                            permissions: toggled_permissions(&directory.permissions, *permission),
                        },
                        terminal,
                        terminal_revision,
                        server_config: config.clone(),
                        providers: providers.clone(),
                    },
                ),
            );
            items.push(
                SelectionItem::new(format!(
                    "{} · {}",
                    permission_title(permission),
                    directory.root.display()
                ))
                .with_id(id)
                .with_columns(
                    permission_title(permission),
                    permission_description(permission),
                    checkbox(enabled),
                ),
            );
        }
    }
    or_empty(items, "No additional directories in this Session")
}

fn all_additional_directory_permissions() -> &'static [WorkspaceAdditionalDirectoryPermissionDto] {
    use WorkspaceAdditionalDirectoryPermissionDto as Permission;
    &[
        Permission::ReadFiles,
        Permission::WriteFiles,
        Permission::ExecuteCommands,
        Permission::WatchFileChanges,
        Permission::LoadProjectConfiguration,
    ]
}

fn toggled_permissions(
    current: &[WorkspaceAdditionalDirectoryPermissionDto],
    permission: WorkspaceAdditionalDirectoryPermissionDto,
) -> Vec<WorkspaceAdditionalDirectoryPermissionDto> {
    use WorkspaceAdditionalDirectoryPermissionDto::ReadFiles;

    let mut permissions = current
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if permissions.contains(&permission) {
        if permission == ReadFiles {
            permissions.clear();
        } else {
            permissions.remove(&permission);
        }
    } else {
        permissions.insert(ReadFiles);
        permissions.insert(permission);
    }
    permissions.into_iter().collect()
}

fn permission_id(permission: &WorkspaceAdditionalDirectoryPermissionDto) -> &'static str {
    match permission {
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles => "read-files",
        WorkspaceAdditionalDirectoryPermissionDto::WriteFiles => "write-files",
        WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands => "execute-commands",
        WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges => "watch-file-changes",
        WorkspaceAdditionalDirectoryPermissionDto::LoadProjectConfiguration => {
            "load-project-configuration"
        }
    }
}

fn permission_title(permission: &WorkspaceAdditionalDirectoryPermissionDto) -> &'static str {
    match permission {
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles => "Read files",
        WorkspaceAdditionalDirectoryPermissionDto::WriteFiles => "Modify files",
        WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands => "Run commands",
        WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges => "Watch file changes",
        WorkspaceAdditionalDirectoryPermissionDto::LoadProjectConfiguration => {
            "Load project configuration"
        }
    }
}

fn permission_description(permission: &WorkspaceAdditionalDirectoryPermissionDto) -> &'static str {
    match permission {
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles => "Allow read_file, grep and glob",
        WorkspaceAdditionalDirectoryPermissionDto::WriteFiles => {
            "Allow file-writing tools; requires Read files"
        }
        WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands => {
            "Permission gate; process tools are not connected yet"
        }
        WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges => {
            "Permission gate; directory watcher is not connected yet"
        }
        WorkspaceAdditionalDirectoryPermissionDto::LoadProjectConfiguration => {
            "Permission gate; project config loading is not connected yet"
        }
    }
}

const fn checkbox(checked: bool) -> &'static str {
    if checked { "[ ✔ ]" } else { "[   ]" }
}

pub(crate) fn provider_api_key_view(provider: String, display_name: String) -> ConfigSelectionView {
    let submit_id = SelectionItemId::new(format!("provider-api-key-submit-{provider}"));
    let actions = BTreeMap::from([(
        submit_id.clone(),
        ConfigSelectionAction::SetProviderApiKey { provider },
    )]);
    ConfigSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                format!("{display_name} API key"),
                vec![SelectionTab::new(
                    "API key",
                    vec![SelectionItem::new(
                        "The key is hidden and stored in the profile secret store",
                    )],
                )],
            )
            .without_tab_bar()
            .without_selection()
            .with_secret_input("Enter API key", submit_id),
            "Enter save  ·  Esc cancel",
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

fn provider_items(
    catalog: &ProviderListResult,
    actions: &mut BTreeMap<SelectionItemId, ConfigSelectionAction>,
) -> Vec<SelectionItem> {
    catalog
        .providers
        .iter()
        .map(|provider| provider_item(provider, actions))
        .collect()
}

fn provider_item(
    provider: &ProviderCatalogEntryDto,
    actions: &mut BTreeMap<SelectionItemId, ConfigSelectionAction>,
) -> SelectionItem {
    let item = SelectionItem::new(&provider.display_name);
    if provider.api_key_policy == ProviderApiKeyPolicyDto::Unsupported {
        return item;
    }
    let id = SelectionItemId::new(format!("provider-api-key-{}", provider.provider));
    actions.insert(
        id.clone(),
        ConfigSelectionAction::OpenProviderApiKey {
            provider: provider.provider.clone(),
            display_name: provider.display_name.clone(),
        },
    );
    item.with_id(id)
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
