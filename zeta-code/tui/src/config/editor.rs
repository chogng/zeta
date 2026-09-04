use crate::config::TerminalSettings;
use crate::models::preferred_model_label;
use crate::status::StatusLineSettings;
use crate::thread::composer::ChatInputMode;
use crate::widgets::list_selection::ListSelection;
use crate::widgets::list_selection::ListSelectionActivationMode;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::list_selection::ListSelectionOutcome;
use crate::widgets::search_box::SearchBoxModel;
use crate::widgets::text_prompt::TextPrompt;
use crate::widgets::text_prompt::TextPromptOutcome;
use crate::widgets::text_prompt::TextPromptSpec;
use std::collections::BTreeMap;
use std::fmt;
use zeroize::Zeroizing;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigEdit {
    pub(crate) terminal: TerminalSettings,
    pub(crate) status_line: StatusLineSettings,
    pub(crate) server_config: ConfigReadResult,
    pub(crate) providers: ProviderListResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConfigSelectionAction {
    SetTerminalSettings(ConfigEdit),
    SetVimMode(ConfigEdit),
    SetShowGitChangesAsDiff(ConfigEdit),
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

pub(crate) struct ConfigChoices {
    pub(crate) model: ListSelectionModel,
    pub(crate) actions: BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
}

pub(crate) struct ProviderApiKeyPrompt {
    pub(crate) spec: TextPromptSpec,
    pub(crate) provider: String,
}

#[derive(Debug)]
pub(crate) struct ConfigEditor {
    selection: ListSelection<ConfigSelectionAction>,
    prompt: Option<ProviderApiKeyPromptState>,
}

#[derive(Debug)]
struct ProviderApiKeyPromptState {
    provider: String,
    prompt: TextPrompt,
    key_hints: crate::widgets::key_hint::KeyHints,
}

#[derive(Debug)]
pub(crate) enum ConfigEditorOutcome {
    Action(ConfigSelectionAction),
    SaveApiKey(ProviderApiKeyEdit),
    Consumed,
    Dismiss,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ConfigEditorPage<'a> {
    Selection(&'a crate::widgets::list_selection::ListSelectionState),
    Prompt(&'a TextPrompt),
}

impl ConfigEditor {
    pub(crate) fn new(spec: ConfigChoices) -> Self {
        Self {
            selection: ListSelection::new(spec.model, spec.actions),
            prompt: None,
        }
    }

    pub(crate) fn replace(&mut self, spec: ConfigChoices) {
        self.selection.replace(spec.model, spec.actions);
    }

    pub(crate) fn close_prompt_and_replace(&mut self, spec: ConfigChoices) {
        self.prompt = None;
        self.replace(spec);
    }

    pub(crate) fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ConfigEditorOutcome {
        if let Some(prompt) = self.prompt.as_mut() {
            return match prompt.prompt.handle_key(key) {
                TextPromptOutcome::Consumed => ConfigEditorOutcome::Consumed,
                TextPromptOutcome::Dismiss => {
                    self.prompt = None;
                    ConfigEditorOutcome::Consumed
                }
                TextPromptOutcome::Submit(value) => ConfigEditorOutcome::SaveApiKey(
                    ProviderApiKeyEdit::new(prompt.provider.clone(), value),
                ),
            };
        }
        match self.selection.handle_key(key) {
            ListSelectionOutcome::Activate(ConfigSelectionAction::OpenProviderApiKey {
                provider,
                display_name,
            }) => {
                self.open_provider_prompt(provider, display_name);
                ConfigEditorOutcome::Consumed
            }
            ListSelectionOutcome::Activate(action) => ConfigEditorOutcome::Action(action),
            ListSelectionOutcome::Adjust(_, _) => ConfigEditorOutcome::Consumed,
            ListSelectionOutcome::Consumed => ConfigEditorOutcome::Consumed,
            ListSelectionOutcome::Dismiss => ConfigEditorOutcome::Dismiss,
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if let Some(prompt) = self.prompt.as_mut() {
            prompt.prompt.handle_paste(pasted);
        } else {
            self.selection.handle_paste(pasted);
        }
    }

    pub(crate) fn page(&self) -> ConfigEditorPage<'_> {
        match &self.prompt {
            Some(prompt) => ConfigEditorPage::Prompt(&prompt.prompt),
            None => ConfigEditorPage::Selection(self.selection.state()),
        }
    }

    pub(crate) fn key_hints(&self) -> &str {
        self.prompt
            .as_ref()
            .map(|prompt| prompt.key_hints.text())
            .unwrap_or_else(|| self.selection.key_hints())
    }

    pub(crate) fn selection(&self) -> Option<&crate::widgets::list_selection::ListSelectionState> {
        self.prompt.is_none().then(|| self.selection.state())
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.prompt.is_none() && self.selection.select_tab(index)
    }

    pub(crate) fn focus_search(&mut self) -> bool {
        self.prompt.is_none() && self.selection.focus_search()
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<ConfigEditorOutcome> {
        let outcome = self.selection.activate_visible_item(index)?;
        Some(match outcome {
            ListSelectionOutcome::Activate(ConfigSelectionAction::OpenProviderApiKey {
                provider,
                display_name,
            }) => {
                self.open_provider_prompt(provider, display_name);
                ConfigEditorOutcome::Consumed
            }
            ListSelectionOutcome::Activate(action) => ConfigEditorOutcome::Action(action),
            ListSelectionOutcome::Adjust(_, _) => ConfigEditorOutcome::Consumed,
            ListSelectionOutcome::Consumed => ConfigEditorOutcome::Consumed,
            ListSelectionOutcome::Dismiss => ConfigEditorOutcome::Dismiss,
        })
    }

    fn open_provider_prompt(&mut self, provider: String, display_name: String) {
        let prompt = provider_api_key_prompt(provider, display_name);
        self.prompt = Some(ProviderApiKeyPromptState {
            provider: prompt.provider,
            prompt: TextPrompt::new(prompt.spec),
            key_hints: crate::widgets::key_hint::KeyHints::new()
                .with_action("Enter", "save")
                .with_action("Esc", "cancel"),
        });
    }
}

pub(crate) fn config_choices(
    config: &ConfigReadResult,
    providers: &ProviderListResult,
    terminal: TerminalSettings,
    status_line: StatusLineSettings,
) -> ConfigChoices {
    let mut actions = BTreeMap::new();
    let mouse_id = ListSelectionItemId::new("terminal-mouse-interactions");
    let mouse_enabled = terminal.mouse_interactions();
    let mut toggled_terminal = terminal;
    toggled_terminal.set_mouse_interactions(!mouse_enabled);
    actions.insert(
        mouse_id.clone(),
        ConfigSelectionAction::SetTerminalSettings(ConfigEdit {
            terminal: toggled_terminal,
            status_line: status_line.clone(),
            server_config: config.clone(),
            providers: providers.clone(),
        }),
    );
    let vim_mode_id = ListSelectionItemId::new("terminal-vim-mode");
    let vim_mode = terminal.input_mode() == ChatInputMode::Vim;
    let mut toggled_terminal = terminal;
    toggled_terminal.set_input_mode(if vim_mode {
        ChatInputMode::Standard
    } else {
        ChatInputMode::Vim
    });
    actions.insert(
        vim_mode_id.clone(),
        ConfigSelectionAction::SetVimMode(ConfigEdit {
            terminal: toggled_terminal,
            status_line: status_line.clone(),
            server_config: config.clone(),
            providers: providers.clone(),
        }),
    );
    let git_changes_id = ListSelectionItemId::new("show-git-changes-as-diff");
    let show_git_changes_as_diff = status_line.show_git_changes_as_diff();
    let mut toggled_status_line = status_line.clone();
    toggled_status_line.set_show_git_changes_as_diff(!show_git_changes_as_diff);
    actions.insert(
        git_changes_id.clone(),
        ConfigSelectionAction::SetShowGitChangesAsDiff(ConfigEdit {
            terminal,
            status_line: toggled_status_line,
            server_config: config.clone(),
            providers: providers.clone(),
        }),
    );
    let mut config_items = vec![
        ListSelectionItem::new("Mouse interactions")
            .with_id(mouse_id)
            .with_columns(
                "Mouse interactions",
                "Select and auto-copy text, click, and hover",
                checkbox(mouse_enabled),
            ),
        ListSelectionItem::new("Vim mode")
            .with_id(vim_mode_id)
            .with_columns(
                "Vim mode",
                "Use Vim editing in ChatInput",
                checkbox(vim_mode),
            ),
        ListSelectionItem::new("Show Git changes as diff")
            .with_id(git_changes_id)
            .with_columns(
                "Show Git changes as diff",
                "Show added and deleted lines instead of changed files",
                checkbox(show_git_changes_as_diff),
            ),
    ];
    config_items.extend(overview(config));
    let provider_items = provider_items(providers, &mut actions);
    ConfigChoices {
        model: ListSelectionModel::new(
            "Config",
            vec![
                ListSelectionGroup::new("Config", config_items),
                ListSelectionGroup::new("Providers", provider_items),
                ListSelectionGroup::new("Language servers", language_servers(config)),
            ],
        )
        .with_activation_mode(ListSelectionActivationMode::EnterOrSpace)
        .with_activation_action("change")
        .with_search(SearchBoxModel::new("Search configuration"))
        .with_empty_message("No matching configuration"),
        actions,
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
        spec: TextPromptSpec {
            title: format!("{display_name} API key"),
            explanation: "The key is hidden and stored in the profile secret store".into(),
            placeholder: "Enter API key".into(),
            masked: true,
        },
        provider,
    }
}

fn overview(config: &ConfigReadResult) -> Vec<ListSelectionItem> {
    vec![
        detail(
            "Preferred model",
            preferred_model_label(config.preferred_model.as_ref()),
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
#[path = "editor_tests.rs"]
mod tests;
