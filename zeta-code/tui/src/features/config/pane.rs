use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use crate::components::text_prompt::TextPromptSpec;
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
    SetTerminalSettings(ConfigEdit),
    SetAdditionalDirectoryPermissions(AdditionalDirectoryPermissionEdit),
    OpenProviderApiKey {
        provider: String,
        display_name: String,
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

pub(crate) struct ConfigPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
}

pub(crate) struct ProviderApiKeyPrompt {
    pub(crate) spec: PaneSpec<TextPromptSpec>,
    pub(crate) provider: String,
}

pub(crate) fn config_pane_spec(
    config: &ConfigReadResult,
    providers: &ProviderListResult,
    terminal: TerminalSettings,
    terminal_revision: u64,
    session_id: &SessionId,
    additional_directories: &WorkspaceAdditionalDirectoryListResult,
) -> ConfigPaneSpec {
    let mut actions = BTreeMap::new();
    let mouse_id = ListSelectionItemId::new("terminal-mouse-interactions");
    let mouse_enabled = terminal.mouse_interactions();
    let mut toggled_terminal = terminal;
    toggled_terminal.set_mouse_interactions(!mouse_enabled);
    actions.insert(
        mouse_id.clone(),
        ConfigSelectionAction::SetTerminalSettings(ConfigEdit {
            terminal: TerminalSettingsEdit {
                expected_revision: terminal_revision,
                settings: toggled_terminal,
            },
            server_config: config.clone(),
            providers: providers.clone(),
            session_id: session_id.clone(),
            additional_directories: additional_directories.clone(),
        }),
    );
    let mut config_items = vec![
        ListSelectionItem::new("Mouse interactions")
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

    ConfigPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Config",
                vec![
                    ListSelectionGroup::new("Config", config_items),
                    ListSelectionGroup::new("Add-dir", permission_items),
                    ListSelectionGroup::new("Providers", provider_items),
                    ListSelectionGroup::new("Language servers", language_servers(config)),
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
    actions: &mut BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
) -> Vec<ListSelectionItem> {
    let mut items = Vec::new();
    let default_permissions = terminal.additional_directory_permissions();
    for permission in all_additional_directory_permissions() {
        let id = ListSelectionItemId::new(format!(
            "additional-directory-default-{}",
            permission_id(permission)
        ));
        let enabled = default_permissions.contains(permission);
        let permissions = toggled_permissions(&default_permissions, *permission);
        let mut settings = terminal;
        settings.set_additional_directory_permissions(&permissions);
        actions.insert(
            id.clone(),
            ConfigSelectionAction::SetTerminalSettings(ConfigEdit {
                terminal: TerminalSettingsEdit {
                    expected_revision: terminal_revision,
                    settings,
                },
                server_config: config.clone(),
                providers: providers.clone(),
                session_id: session_id.clone(),
                additional_directories: snapshot.clone(),
            }),
        );
        items.push(
            ListSelectionItem::new(permission_title(permission))
                .with_id(id)
                .with_columns(permission_title(permission), "", checkbox(enabled)),
        );
    }
    for (directory_index, directory) in snapshot.directories.iter().enumerate() {
        for permission in all_additional_directory_permissions() {
            let id = ListSelectionItemId::new(format!(
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
                ListSelectionItem::new(format!(
                    "{} · {}",
                    permission_title(permission),
                    directory.root.display()
                ))
                .with_id(id)
                .with_columns(
                    format!(
                        "{} · {}",
                        permission_title(permission),
                        directory.root.display()
                    ),
                    permission_description(permission, directory),
                    checkbox(enabled),
                ),
            );
        }
    }
    items
}

fn all_additional_directory_permissions() -> &'static [WorkspaceAdditionalDirectoryPermissionDto] {
    use WorkspaceAdditionalDirectoryPermissionDto as Permission;
    &[
        Permission::ReadFiles,
        Permission::WriteFiles,
        Permission::ExecuteCommands,
        Permission::WatchFileChanges,
        Permission::UseWorkspaceFiles,
        Permission::UseWorkspaceSearch,
        Permission::LoadInstructionsAndAgents,
        Permission::DiscoverSkills,
        Permission::DiscoverMcp,
        Permission::UseLanguageServices,
        Permission::DiscoverHooks,
        Permission::DiscoverPlugins,
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
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceFiles => "workspace-files",
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch => "workspace-search",
        WorkspaceAdditionalDirectoryPermissionDto::LoadInstructionsAndAgents => {
            "instructions-and-agents"
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverSkills => "skills",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverMcp => "mcp",
        WorkspaceAdditionalDirectoryPermissionDto::UseLanguageServices => "lsp",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverHooks => "hooks",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverPlugins => "plugins",
    }
}

fn permission_title(permission: &WorkspaceAdditionalDirectoryPermissionDto) -> &'static str {
    match permission {
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles => "Read files",
        WorkspaceAdditionalDirectoryPermissionDto::WriteFiles => "Modify files",
        WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands => "Run commands",
        WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges => "Watch file changes",
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceFiles => "Workspace Files",
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch => "Workspace Search",
        WorkspaceAdditionalDirectoryPermissionDto::LoadInstructionsAndAgents => {
            "Instructions & Agents"
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverSkills => "Skills",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverMcp => "MCP",
        WorkspaceAdditionalDirectoryPermissionDto::UseLanguageServices => "LSP",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverHooks => "Hooks",
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverPlugins => "Plugins",
    }
}

fn permission_description(
    permission: &WorkspaceAdditionalDirectoryPermissionDto,
    directory: &zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryDto,
) -> String {
    match permission {
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles => {
            "Allow read_file, grep and glob".into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::WriteFiles => {
            "Allow file-writing tools and apply_patch; requires Read files".into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands => {
            "Allow shell-command and Session terminals; requires Read files".into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges => {
            "Refresh authorized project configuration after file changes; requires Read files"
                .into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceFiles => {
            "Show this directory in Workspace Files; requires Read files".into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch => {
            "Search this directory from Workspace Search; requires Read files".into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::LoadInstructionsAndAgents => {
            "Load .zeta/instructions and .zeta/agents; requires Read files".into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverSkills => {
            format!(
                "Discover Skills from this directory ({} found); requires Read files",
                directory.contributions.skills.len()
            )
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverMcp => {
            format!(
                "Authorize MCP declarations ({} found); connect them separately",
                directory.contributions.mcp_servers.len()
            )
        }
        WorkspaceAdditionalDirectoryPermissionDto::UseLanguageServices => {
            "Use language servers for this directory; starting them also requires Run commands"
                .into()
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverHooks => {
            format!(
                "Discover Hooks ({} found); running them also requires Run commands",
                directory.contributions.hooks.len()
            )
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverPlugins => {
            format!(
                "Authorize Plugin requests ({} found); installation stays separate",
                directory.contributions.plugins.len()
            )
        }
    }
}

const fn checkbox(checked: bool) -> &'static str {
    if checked { "[ ✔ ]" } else { "[   ]" }
}

pub(crate) fn provider_api_key_prompt(
    provider: String,
    display_name: String,
) -> ProviderApiKeyPrompt {
    ProviderApiKeyPrompt {
        spec: PaneSpec::new(
            TextPromptSpec {
                title: format!("{display_name} API key"),
                explanation: "The key is hidden and stored in the profile secret store".into(),
                placeholder: "Enter API key".into(),
                masked: true,
            },
            "Enter save  ·  Esc cancel",
        ),
        provider,
    }
}

fn overview(config: &ConfigReadResult) -> Vec<ListSelectionItem> {
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
    actions: &mut BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
) -> Vec<ListSelectionItem> {
    catalog
        .providers
        .iter()
        .map(|provider| provider_item(provider, actions))
        .collect()
}

fn provider_item(
    provider: &ProviderCatalogEntryDto,
    actions: &mut BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
) -> ListSelectionItem {
    let item = ListSelectionItem::new(&provider.display_name);
    if provider.api_key_policy == ProviderApiKeyPolicyDto::Unsupported {
        return item;
    }
    let id = ListSelectionItemId::new(format!("provider-api-key-{}", provider.provider));
    actions.insert(
        id.clone(),
        ConfigSelectionAction::OpenProviderApiKey {
            provider: provider.provider.clone(),
            display_name: provider.display_name.clone(),
        },
    );
    item.with_id(id)
}

fn language_servers(config: &ConfigReadResult) -> Vec<ListSelectionItem> {
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

fn detail(label: &str, value: impl Into<String>) -> ListSelectionItem {
    ListSelectionItem::new(label).with_description(value)
}

fn or_empty(items: Vec<ListSelectionItem>, message: &str) -> Vec<ListSelectionItem> {
    if items.is_empty() {
        vec![ListSelectionItem::new(message)]
    } else {
        items
    }
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
