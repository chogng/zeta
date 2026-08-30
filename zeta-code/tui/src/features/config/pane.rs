use crate::components::chat_input::ChatInputMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use crate::components::text_prompt::TextPromptSpec;
use crate::features::config::FollowUpMode;
use crate::features::config::TerminalSettings;
use crate::features::config::TerminalSettingsEdit;
use crate::features::config::preferred_model;
use std::collections::BTreeMap;
use std::fmt;
use zeroize::Zeroizing;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListResult;
use zeta_app_server_protocol::protocol::environment::SessionDirPermissionsSetParams;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};
use zeta_protocol::SessionId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigEdit {
    pub(crate) terminal: TerminalSettingsEdit,
    pub(crate) server_config: ConfigReadResult,
    pub(crate) providers: ProviderListResult,
    pub(crate) session_id: SessionId,
    pub(crate) dirs: SessionDirListResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PermissionEdit {
    pub(crate) params: SessionDirPermissionsSetParams,
    pub(crate) terminal: TerminalSettings,
    pub(crate) terminal_revision: u64,
    pub(crate) server_config: ConfigReadResult,
    pub(crate) providers: ProviderListResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConfigSelectionAction {
    SetTerminalSettings(ConfigEdit),
    ChooseFollowUpMode {
        queue: Box<ConfigEdit>,
        steer: Box<ConfigEdit>,
    },
    ChooseInputMode {
        standard: Box<ConfigEdit>,
        vim: Box<ConfigEdit>,
    },
    SetPermissions(PermissionEdit),
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
    dirs: &SessionDirListResult,
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
            dirs: dirs.clone(),
        }),
    );
    let input_mode_id = ListSelectionItemId::new("terminal-input-mode");
    let mut standard_terminal = terminal;
    standard_terminal.set_input_mode(ChatInputMode::Standard);
    let mut vim_terminal = terminal;
    vim_terminal.set_input_mode(ChatInputMode::Vim);
    actions.insert(
        input_mode_id.clone(),
        ConfigSelectionAction::ChooseInputMode {
            standard: Box::new(ConfigEdit {
                terminal: TerminalSettingsEdit {
                    expected_revision: terminal_revision,
                    settings: standard_terminal,
                },
                server_config: config.clone(),
                providers: providers.clone(),
                session_id: session_id.clone(),
                dirs: dirs.clone(),
            }),
            vim: Box::new(ConfigEdit {
                terminal: TerminalSettingsEdit {
                    expected_revision: terminal_revision,
                    settings: vim_terminal,
                },
                server_config: config.clone(),
                providers: providers.clone(),
                session_id: session_id.clone(),
                dirs: dirs.clone(),
            }),
        },
    );
    let follow_up_id = ListSelectionItemId::new("terminal-follow-up-mode");
    let mut queue_terminal = terminal;
    queue_terminal.set_follow_up_mode(FollowUpMode::Queue);
    let mut steer_terminal = terminal;
    steer_terminal.set_follow_up_mode(FollowUpMode::Steer);
    actions.insert(
        follow_up_id.clone(),
        ConfigSelectionAction::ChooseFollowUpMode {
            queue: Box::new(ConfigEdit {
                terminal: TerminalSettingsEdit {
                    expected_revision: terminal_revision,
                    settings: queue_terminal,
                },
                server_config: config.clone(),
                providers: providers.clone(),
                session_id: session_id.clone(),
                dirs: dirs.clone(),
            }),
            steer: Box::new(ConfigEdit {
                terminal: TerminalSettingsEdit {
                    expected_revision: terminal_revision,
                    settings: steer_terminal,
                },
                server_config: config.clone(),
                providers: providers.clone(),
                session_id: session_id.clone(),
                dirs: dirs.clone(),
            }),
        },
    );
    let mut config_items = vec![
        ListSelectionItem::new("Mouse interactions")
            .with_id(mouse_id)
            .with_columns(
                "Mouse interactions",
                "Select and auto-copy text, click, and hover",
                checkbox(mouse_enabled),
            ),
        ListSelectionItem::new("Follow-up messages")
            .with_id(follow_up_id)
            .with_columns(
                "Follow-up messages",
                "How Enter sends a message while a Turn is running",
                match terminal.follow_up_mode() {
                    FollowUpMode::Queue => "Queue",
                    FollowUpMode::Steer => "Steer",
                },
            ),
        ListSelectionItem::new("Input mode")
            .with_id(input_mode_id)
            .with_columns(
                "Input mode",
                "Standard or Vim editing inside ChatInput",
                match terminal.input_mode() {
                    ChatInputMode::Standard => "Standard",
                    ChatInputMode::Vim => "Vim",
                },
            ),
    ];
    config_items.extend(overview(config));
    let provider_items = provider_items(providers, &mut actions);
    let permission_items = dir_permission_items(
        dirs,
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
            "Space search  ·  Tab/Shift-Tab tabs  ·  ←/→ change  ·  Enter select/toggle  ·  ↑/↓ inspect  ·  Esc back",
        ),
        actions,
    }
}

fn dir_permission_items(
    snapshot: &SessionDirListResult,
    session_id: &SessionId,
    terminal: TerminalSettings,
    terminal_revision: u64,
    config: &ConfigReadResult,
    providers: &ProviderListResult,
    actions: &mut BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
) -> Vec<ListSelectionItem> {
    let mut items = Vec::new();
    let default_permissions = terminal.dir_permissions();
    for permission in all_dir_permissions() {
        let id = ListSelectionItemId::new(format!("dir-default-{}", permission_id(permission)));
        let enabled = default_permissions.contains(permission);
        let permissions = toggled_permissions(&default_permissions, *permission);
        let mut settings = terminal;
        settings.set_dir_permissions(&permissions);
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
                dirs: snapshot.clone(),
            }),
        );
        items.push(
            ListSelectionItem::new(permission_title(permission))
                .with_id(id)
                .with_columns(permission_title(permission), "", checkbox(enabled)),
        );
    }
    for (directory_index, directory) in snapshot.dirs.iter().enumerate() {
        for permission in all_dir_permissions() {
            let id = ListSelectionItemId::new(format!(
                "dir-{directory_index}-{}",
                permission_id(permission)
            ));
            let enabled = directory.permissions.contains(permission);
            actions.insert(
                id.clone(),
                ConfigSelectionAction::SetPermissions(PermissionEdit {
                    params: SessionDirPermissionsSetParams {
                        session_id: session_id.clone(),
                        path: directory.path.clone(),
                        expected_revision: snapshot.revision,
                        permissions: toggled_permissions(&directory.permissions, *permission),
                    },
                    terminal,
                    terminal_revision,
                    server_config: config.clone(),
                    providers: providers.clone(),
                }),
            );
            items.push(
                ListSelectionItem::new(format!(
                    "{} · {}",
                    permission_title(permission),
                    directory.path.display()
                ))
                .with_id(id)
                .with_columns(
                    format!(
                        "{} · {}",
                        permission_title(permission),
                        directory.path.display()
                    ),
                    permission_description(permission, directory),
                    checkbox(enabled),
                ),
            );
        }
    }
    items
}

fn all_dir_permissions() -> &'static [PermissionDto] {
    use PermissionDto as Permission;
    &[
        Permission::ReadFiles,
        Permission::WriteFiles,
        Permission::ExecuteCommands,
        Permission::WatchFiles,
        Permission::BrowseFiles,
        Permission::SearchFiles,
        Permission::LoadInstructions,
        Permission::LoadConfig,
        Permission::DiscoverSkills,
        Permission::DiscoverMcp,
        Permission::UseLanguageServices,
        Permission::DiscoverHooks,
        Permission::DiscoverPlugins,
        Permission::InspectRepository,
        Permission::MutateRepository,
    ]
}

fn toggled_permissions(current: &[PermissionDto], permission: PermissionDto) -> Vec<PermissionDto> {
    let mut permissions = current
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if permissions.contains(&permission) {
        permissions.remove(&permission);
    } else {
        permissions.insert(permission);
    }
    permissions.into_iter().collect()
}

fn permission_id(permission: &PermissionDto) -> &'static str {
    match permission {
        PermissionDto::ReadFiles => "read-files",
        PermissionDto::WriteFiles => "write-files",
        PermissionDto::ExecuteCommands => "execute-commands",
        PermissionDto::WatchFiles => "watch-files",
        PermissionDto::BrowseFiles => "browse-files",
        PermissionDto::SearchFiles => "search-files",
        PermissionDto::LoadInstructions => "load-instructions",
        PermissionDto::LoadConfig => "load-config",
        PermissionDto::DiscoverSkills => "skills",
        PermissionDto::DiscoverMcp => "mcp",
        PermissionDto::UseLanguageServices => "lsp",
        PermissionDto::DiscoverHooks => "hooks",
        PermissionDto::DiscoverPlugins => "plugins",
        PermissionDto::InspectRepository => "inspect-repository",
        PermissionDto::MutateRepository => "mutate-repository",
    }
}

fn permission_title(permission: &PermissionDto) -> &'static str {
    match permission {
        PermissionDto::ReadFiles => "Read files",
        PermissionDto::WriteFiles => "Modify files",
        PermissionDto::ExecuteCommands => "Run commands",
        PermissionDto::WatchFiles => "Watch file changes",
        PermissionDto::BrowseFiles => "Browse files",
        PermissionDto::SearchFiles => "Search files",
        PermissionDto::LoadInstructions => "Load instructions",
        PermissionDto::LoadConfig => "Load config",
        PermissionDto::DiscoverSkills => "Skills",
        PermissionDto::DiscoverMcp => "MCP",
        PermissionDto::UseLanguageServices => "LSP",
        PermissionDto::DiscoverHooks => "Hooks",
        PermissionDto::DiscoverPlugins => "Plugins",
        PermissionDto::InspectRepository => "Inspect repository",
        PermissionDto::MutateRepository => "Mutate repository",
    }
}

fn permission_description(
    permission: &PermissionDto,
    directory: &zeta_app_server_protocol::protocol::environment::SessionDirDto,
) -> String {
    match permission {
        PermissionDto::ReadFiles => "Allow read_file, grep and glob".into(),
        PermissionDto::WriteFiles => "Allow file-writing tools and apply_patch".into(),
        PermissionDto::ExecuteCommands => "Allow shell-command and Session terminals".into(),
        PermissionDto::WatchFiles => "Watch this directory for file changes".into(),
        PermissionDto::BrowseFiles => "Show this directory in file browsing surfaces".into(),
        PermissionDto::SearchFiles => "Search file contents in this directory".into(),
        PermissionDto::LoadInstructions => "Load .zeta/instructions and .zeta/agents".into(),
        PermissionDto::LoadConfig => "Load configuration supplied by this directory".into(),
        PermissionDto::DiscoverSkills => {
            format!(
                "Discover Skills from this directory ({} found); requires Read files",
                directory.contributions.skills.len()
            )
        }
        PermissionDto::DiscoverMcp => {
            format!(
                "Authorize MCP declarations ({} found); connect them separately",
                directory.contributions.mcp_servers.len()
            )
        }
        PermissionDto::UseLanguageServices => {
            "Use language servers for this directory; starting them also requires Run commands"
                .into()
        }
        PermissionDto::DiscoverHooks => {
            format!(
                "Discover Hooks ({} found); running them also requires Run commands",
                directory.contributions.hooks.len()
            )
        }
        PermissionDto::DiscoverPlugins => {
            format!(
                "Authorize Plugin requests ({} found); installation stays separate",
                directory.contributions.plugins.len()
            )
        }
        PermissionDto::InspectRepository => "Read repository metadata and status".into(),
        PermissionDto::MutateRepository => "Change repository state".into(),
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
