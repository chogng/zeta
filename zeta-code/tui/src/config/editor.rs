use crate::config::TerminalSettings;
use crate::keymap::bindings;
use crate::nls;
use crate::nls::Language;
use crate::nls::Message;
use crate::status::StatusLineSettings;
use crate::thread::composer::ChatInputMode;
use crate::widgets::list_selection::ListSelection;
use crate::widgets::list_selection::ListSelectionAdjustment;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;
use crate::widgets::list_selection::ListSelectionOutcome;
use crate::widgets::list_selection::ListSelectionSpec;
use crate::widgets::search_box::SearchBoxModel;
use crate::widgets::text_prompt::TextPrompt;
use crate::widgets::text_prompt::TextPromptOutcome;
use crate::widgets::text_prompt::TextPromptSpec;
use std::collections::BTreeMap;
use std::fmt;
use zeroize::Zeroizing;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigDto;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::provider::{
    ProviderApiKeyPolicyDto, ProviderCatalogEntryDto, ProviderListResult,
};

const ISSUE_MODEL_ROW: &str = "issue-analysis-model";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigEdit {
    pub(crate) terminal: TerminalSettings,
    pub(crate) status_line: StatusLineSettings,
    pub(crate) server_config: ConfigReadResult,
    pub(crate) providers: ProviderListResult,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConfigSelectionAction {
    SetIssues(super::IssueConfigEdit),
    OpenIssueModels {
        expected_revision: u64,
    },
    OpenOpenAi(super::openai::Settings),
    Connection(super::openai::Request),
    OpenSubscription,
    Subscription(super::SubscriptionCommand),
    SetTerminalSettings(ConfigEdit),
    SetVimMode(ConfigEdit),
    SetShowGitChangesAsDiff(ConfigEdit),
    SetLanguage(ConfigEdit),
    SetLanguageServerMode(LanguageServerEdit),
    OpenProviderApiKey {
        provider: String,
        display_name: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LanguageServerEdit {
    pub(crate) expected_revision: u64,
    pub(crate) server_id: String,
    pub(crate) config: LanguageServerConfigDto,
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

pub(crate) type ConfigChoices = ListSelectionSpec<ConfigSelectionAction>;

pub(crate) struct ProviderApiKeyPrompt {
    pub(crate) spec: TextPromptSpec,
    pub(crate) provider: String,
}

#[derive(Debug)]
pub(crate) struct ConfigEditor {
    revision: u64,
    issue_models: Option<ListSelection<ConfigSelectionAction>>,
    issue_model_request: Option<zeta_protocol::CommandId>,
    selection: ListSelection<ConfigSelectionAction>,
    openai: Option<super::openai::Panel>,
    subscription: Option<ListSelection<ConfigSelectionAction>>,
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
    LoadIssueModels {
        request_id: zeta_protocol::CommandId,
        expected_revision: u64,
    },
    Action(ConfigSelectionAction),
    SaveApiKey(ProviderApiKeyEdit),
    Consumed,
    Dismiss,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ConfigEditorPage<'a> {
    Selection(&'a crate::widgets::list_selection::ListSelectionState),
    Prompt(&'a TextPrompt),
    OpenAi(&'a super::openai::Panel),
}

impl ConfigEditor {
    pub(crate) fn new(spec: ConfigChoices) -> Self {
        Self {
            revision: config_revision(&spec),
            issue_models: None,
            issue_model_request: None,
            selection: ListSelection::new(spec.model, spec.actions),
            openai: None,
            subscription: None,
            prompt: None,
        }
    }

    pub(crate) fn replace(&mut self, spec: ConfigChoices) {
        let revision = config_revision(&spec);
        if revision < self.revision {
            return;
        }
        if revision != self.revision
            || !spec
                .actions
                .values()
                .any(|action| matches!(action, ConfigSelectionAction::OpenIssueModels { .. }))
        {
            self.issue_models = None;
            self.issue_model_request = None;
        }
        self.revision = revision;
        if let Some(openai) = &mut self.openai {
            if let Some(ConfigSelectionAction::OpenOpenAi(settings)) = spec
                .actions
                .values()
                .find(|action| matches!(action, ConfigSelectionAction::OpenOpenAi(_)))
            {
                openai.replace(settings.clone());
            }
        }
        self.selection.replace(spec.model, spec.actions);
    }

    pub(crate) fn close_prompt_and_replace(&mut self, spec: ConfigChoices) {
        self.prompt = None;
        self.replace(spec);
    }

    pub(crate) fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ConfigEditorOutcome {
        if let Some(models) = self.issue_models.as_mut() {
            return match models.handle_key(key) {
                ListSelectionOutcome::Activate(action) => {
                    self.issue_models = None;
                    ConfigEditorOutcome::Action(action)
                }
                ListSelectionOutcome::Dismiss => {
                    self.issue_models = None;
                    ConfigEditorOutcome::Consumed
                }
                _ => ConfigEditorOutcome::Consumed,
            };
        }
        if let Some(subscription) = self.subscription.as_mut() {
            let outcome = subscription.handle_key(key);
            return self.handle_subscription_outcome(outcome);
        }
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
        if let Some(openai) = &mut self.openai {
            let outcome = openai.handle_key(key);
            if matches!(outcome, ConfigEditorOutcome::Dismiss) {
                self.openai = None;
                return ConfigEditorOutcome::Consumed;
            }
            return outcome;
        }
        let outcome = self.selection.handle_key(key);
        self.handle_selection_outcome(outcome)
    }

    fn handle_selection_outcome(
        &mut self,
        outcome: ListSelectionOutcome<ConfigSelectionAction>,
    ) -> ConfigEditorOutcome {
        match outcome {
            ListSelectionOutcome::Activate(ConfigSelectionAction::OpenIssueModels {
                expected_revision,
            }) => {
                let request_id = crate::client::new_command_id("issue-models");
                self.issue_model_request = Some(request_id.clone());
                ConfigEditorOutcome::LoadIssueModels {
                    request_id,
                    expected_revision,
                }
            }
            ListSelectionOutcome::Activate(ConfigSelectionAction::SetLanguage(edit)) => {
                language_outcome(edit, ListSelectionAdjustment::Next)
            }
            ListSelectionOutcome::Activate(ConfigSelectionAction::OpenOpenAi(settings)) => {
                self.openai = Some(super::openai::Panel::new(settings));
                ConfigEditorOutcome::Consumed
            }
            ListSelectionOutcome::Activate(ConfigSelectionAction::OpenProviderApiKey {
                provider,
                display_name,
            }) => {
                self.open_provider_prompt(provider, display_name);
                ConfigEditorOutcome::Consumed
            }
            ListSelectionOutcome::Activate(action) => ConfigEditorOutcome::Action(action),
            ListSelectionOutcome::Adjust(action, adjustment) => match action {
                ConfigSelectionAction::OpenIssueModels { .. }
                | ConfigSelectionAction::OpenProviderApiKey { .. }
                | ConfigSelectionAction::OpenOpenAi(_)
                | ConfigSelectionAction::Connection(_)
                | ConfigSelectionAction::OpenSubscription
                | ConfigSelectionAction::Subscription(_) => ConfigEditorOutcome::Consumed,
                ConfigSelectionAction::SetLanguage(edit) => language_outcome(edit, adjustment),
                action => ConfigEditorOutcome::Action(action),
            },
            ListSelectionOutcome::Consumed | ListSelectionOutcome::FocusPrevious => {
                ConfigEditorOutcome::Consumed
            }
            ListSelectionOutcome::Dismiss => {
                if self.openai.take().is_some() {
                    ConfigEditorOutcome::Consumed
                } else {
                    ConfigEditorOutcome::Dismiss
                }
            }
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if let Some(models) = self.issue_models.as_mut() {
            models.handle_paste(pasted);
            return;
        }
        if let Some(subscription) = self.subscription.as_mut() {
            subscription.handle_paste(pasted);
        } else if let Some(prompt) = self.prompt.as_mut() {
            prompt.prompt.handle_paste(pasted);
        } else if let Some(openai) = self.openai.as_mut() {
            openai.handle_paste(pasted);
        } else {
            self.selection.handle_paste(pasted);
        }
    }

    pub(crate) fn page(&self) -> ConfigEditorPage<'_> {
        if let Some(models) = &self.issue_models {
            return ConfigEditorPage::Selection(models.state());
        }
        if let Some(subscription) = &self.subscription {
            return ConfigEditorPage::Selection(subscription.state());
        }
        match &self.prompt {
            Some(prompt) => ConfigEditorPage::Prompt(&prompt.prompt),
            None => self.openai.as_ref().map_or_else(
                || ConfigEditorPage::Selection(self.selection.state()),
                ConfigEditorPage::OpenAi,
            ),
        }
    }

    pub(crate) fn key_hints(&self) -> &str {
        if let Some(models) = &self.issue_models {
            return models.key_hints();
        }
        if let Some(subscription) = &self.subscription {
            return subscription.key_hints();
        }
        self.prompt
            .as_ref()
            .map(|prompt| prompt.key_hints.text())
            .unwrap_or_else(|| {
                if let Some(openai) = &self.openai {
                    openai.key_hints()
                } else {
                    self.selection.key_hints()
                }
            })
    }

    pub(crate) fn selection(&self) -> Option<&crate::widgets::list_selection::ListSelectionState> {
        if let Some(models) = &self.issue_models {
            return Some(models.state());
        }
        if let Some(subscription) = &self.subscription {
            return Some(subscription.state());
        }
        (self.prompt.is_none() && self.openai.is_none()).then(|| self.selection.state())
    }

    pub(crate) fn finish_issue_models(
        &mut self,
        request_id: zeta_protocol::CommandId,
        result: Result<ConfigChoices, String>,
    ) {
        if self.issue_model_request.as_ref() != Some(&request_id) {
            return;
        }
        self.issue_model_request = None;
        if self.selection.state().selected_item().and_then(ListSelectionItem::id)
            != Some(&ListSelectionItemId::new(ISSUE_MODEL_ROW)) {
            return;
        }
        match result {
            Ok(spec) if config_revision(&spec) == self.revision => {
                self.issue_models = Some(ListSelection::new(spec.model, spec.actions))
            }
            Ok(_) => self
                .selection
                .state_mut()
                .set_message(Some("Configuration changed; reopen model selection".into())),
            Err(error) => self.selection.state_mut().set_message(Some(error)),
        }
    }

    pub(crate) fn open_subscription(&mut self, spec: ConfigChoices) {
        if let Some(openai) = &mut self.openai {
            openai.update_subscription(spec);
        } else {
            self.subscription = Some(ListSelection::new(spec.model, spec.actions));
        }
    }

    pub(crate) fn complete_connection(&mut self, reply: super::openai::Reply) {
        if let Ok((choices, _)) = &reply.result {
            let revision = config_revision(choices);
            if revision >= self.revision {
                self.revision = revision;
                self.selection
                    .replace(choices.model.clone(), choices.actions.clone());
            }
        }
        if let Some(panel) = &mut self.openai {
            panel.complete(reply);
        }
    }

    pub(crate) fn update_subscription(&mut self, spec: ConfigChoices) {
        if let Some(openai) = &mut self.openai {
            openai.update_subscription(spec);
            return;
        }
        if let Some(subscription) = self.subscription.as_mut() {
            subscription.replace(spec.model, spec.actions);
        }
    }

    fn handle_subscription_outcome(
        &mut self,
        outcome: ListSelectionOutcome<ConfigSelectionAction>,
    ) -> ConfigEditorOutcome {
        match outcome {
            ListSelectionOutcome::Activate(action) => ConfigEditorOutcome::Action(action),
            ListSelectionOutcome::Dismiss => {
                self.subscription = None;
                ConfigEditorOutcome::Consumed
            }
            _ => ConfigEditorOutcome::Consumed,
        }
    }

    fn open_provider_prompt(&mut self, provider: String, display_name: String) {
        let prompt = provider_api_key_prompt(provider, display_name);
        self.prompt = Some(ProviderApiKeyPromptState {
            provider: prompt.provider,
            prompt: TextPrompt::new(prompt.spec),
            key_hints: crate::widgets::key_hint::KeyHints::new()
                .with_binding(bindings::SAVE)
                .with_binding(bindings::CANCEL),
        });
    }
}

fn config_revision(choices: &ConfigChoices) -> u64 {
    choices
        .actions
        .values()
        .find_map(|action| match action {
            ConfigSelectionAction::OpenOpenAi(settings) => Some(settings.revision),
            ConfigSelectionAction::SetIssues(edit) => Some(edit.expected_revision),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn config_choices(
    config: &ConfigReadResult,
    providers: &ProviderListResult,
    terminal: TerminalSettings,
    status_line: StatusLineSettings,
) -> ConfigChoices {
    let mut actions = BTreeMap::new();
    let language = terminal.language();
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
    let language_id = ListSelectionItemId::new("language");
    actions.insert(
        language_id.clone(),
        ConfigSelectionAction::SetLanguage(ConfigEdit {
            terminal,
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
    let memory_diagnostics_id = ListSelectionItemId::new("memory-diagnostics");
    let memory_diagnostics = terminal.memory_diagnostics();
    let mut toggled_terminal = terminal;
    toggled_terminal.set_memory_diagnostics(!memory_diagnostics);
    actions.insert(
        memory_diagnostics_id.clone(),
        ConfigSelectionAction::SetTerminalSettings(ConfigEdit {
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
    let issue_merge_id = ListSelectionItemId::new("issue-merge-recommendations");
    let mut toggled_issues = config.issues.clone();
    toggled_issues.recommend_merge = !toggled_issues.recommend_merge;
    actions.insert(
        issue_merge_id.clone(),
        ConfigSelectionAction::SetIssues(super::IssueConfigEdit {
            expected_revision: config.revision,
            config: toggled_issues,
        }),
    );
    let issue_model_id = ListSelectionItemId::new(ISSUE_MODEL_ROW);
    if config.issues.recommend_merge {
        actions.insert(
            issue_model_id.clone(),
            ConfigSelectionAction::OpenIssueModels {
                expected_revision: config.revision,
            },
        );
    }
    let model_label = config
        .issues
        .analysis_model
        .as_ref()
        .map(|model| format!("{}/{}", model.provider, model.model))
        .unwrap_or_else(|| nls::text(language, Message::ConfigIssueModelMissing).into());
    let issue_items = vec![
        ListSelectionItem::new(nls::text(language, Message::ConfigIssueModel))
            .with_id(issue_model_id)
            .with_columns(
                nls::text(language, Message::ConfigIssueModel),
                nls::text(language, Message::ConfigIssueModelDescription),
                model_label,
            ),
    ];
    let issue_tab =
        ListSelectionGroup::new(nls::text(language, Message::ConfigIssues), issue_items);
    let issue_tab = if config.issues.recommend_merge {
        issue_tab
    } else {
        issue_tab.disabled()
    };
    let config_items = vec![
        ListSelectionItem::new(nls::text(language, Message::ConfigEnhancedTui))
            .with_id(mouse_id)
            .with_columns(
                nls::text(language, Message::ConfigEnhancedTui),
                nls::text(language, Message::ConfigEnhancedTuiDescription),
                checkbox(mouse_enabled),
            ),
        ListSelectionItem::new(nls::text(language, Message::ConfigVimMode))
            .with_id(vim_mode_id)
            .with_columns(
                nls::text(language, Message::ConfigVimMode),
                nls::text(language, Message::ConfigVimModeDescription),
                checkbox(vim_mode),
            ),
        ListSelectionItem::new(nls::text(language, Message::ConfigMemoryDiagnostics))
            .with_id(memory_diagnostics_id)
            .with_columns(
                nls::text(language, Message::ConfigMemoryDiagnostics),
                nls::text(language, Message::ConfigMemoryDiagnosticsDescription),
                checkbox(memory_diagnostics),
            ),
        ListSelectionItem::new(nls::text(language, Message::ConfigGitChangesAsDiff))
            .with_id(git_changes_id)
            .with_columns(
                nls::text(language, Message::ConfigGitChangesAsDiff),
                nls::text(language, Message::ConfigGitChangesAsDiffDescription),
                checkbox(show_git_changes_as_diff),
            ),
        ListSelectionItem::new(nls::text(language, Message::ConfigLanguage))
            .with_id(language_id)
            .with_columns(
                nls::text(language, Message::ConfigLanguage),
                nls::text(language, Message::ConfigLanguageDescription),
                language.label(),
            ),
        ListSelectionItem::new(nls::text(language, Message::ConfigIssueMerge))
            .with_id(issue_merge_id)
            .with_columns(
                nls::text(language, Message::ConfigIssueMerge),
                nls::text(language, Message::ConfigIssueMergeDescription),
                checkbox(config.issues.recommend_merge),
            ),
    ];
    let provider_items = provider_items(config, providers, &mut actions);
    let language_server_items = language_servers(config, language, &mut actions);
    ConfigChoices {
        model: ListSelectionModel::new(
            nls::text(language, Message::ConfigTitle),
            vec![
                ListSelectionGroup::new(nls::text(language, Message::ConfigTitle), config_items),
                ListSelectionGroup::new(
                    nls::text(language, Message::ConfigProviders),
                    provider_items,
                ),
                ListSelectionGroup::new(
                    nls::text(language, Message::ConfigLanguageServers),
                    language_server_items,
                ),
                issue_tab,
            ],
        )
        .with_activation(bindings::CONFIG_CHANGE)
        .with_search(SearchBoxModel::new(nls::text(
            language,
            Message::ConfigSearch,
        )))
        .with_empty_message(nls::text(language, Message::ConfigNoMatches)),
        actions,
    }
}

fn language_outcome(
    mut edit: ConfigEdit,
    adjustment: ListSelectionAdjustment,
) -> ConfigEditorOutcome {
    let language = match adjustment {
        ListSelectionAdjustment::Previous => edit.terminal.language().previous(),
        ListSelectionAdjustment::Next => edit.terminal.language().next(),
    };
    edit.terminal.set_language(language);
    ConfigEditorOutcome::Action(ConfigSelectionAction::SetLanguage(edit))
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

fn provider_items(
    config: &ConfigReadResult,
    catalog: &ProviderListResult,
    actions: &mut BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
) -> Vec<ListSelectionItem> {
    let mut items: Vec<_> = catalog
        .providers
        .iter()
        .filter(|provider| {
            !matches!(
                provider.provider.as_str(),
                "openai" | "openai-chatgpt" | "openai-compatible"
            ) && !provider.provider.starts_with("custom-")
        })
        .map(|provider| provider_item(provider, actions))
        .collect();
    let id = ListSelectionItemId::new("openai");
    actions.insert(
        id.clone(),
        ConfigSelectionAction::OpenOpenAi(super::openai::Settings::new(config, catalog)),
    );
    items.insert(0, ListSelectionItem::new("OpenAI").with_id(id));
    items
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

fn language_servers(
    config: &ConfigReadResult,
    language: Language,
    actions: &mut BTreeMap<ListSelectionItemId, ConfigSelectionAction>,
) -> Vec<ListSelectionItem> {
    or_empty(
        config
            .language_servers
            .iter()
            .map(|(server_id, server)| {
                let enabled = server.mode == LanguageServerModeDto::Enabled;
                let id = ListSelectionItemId::new(format!("language-server-{server_id}"));
                let mut next_config = server.clone();
                next_config.mode = if enabled {
                    LanguageServerModeDto::Disabled
                } else {
                    LanguageServerModeDto::Enabled
                };
                actions.insert(
                    id.clone(),
                    ConfigSelectionAction::SetLanguageServerMode(LanguageServerEdit {
                        expected_revision: config.revision,
                        server_id: server_id.clone(),
                        config: next_config,
                    }),
                );
                let description = server.executable.as_deref().unwrap_or_default();
                ListSelectionItem::new(server_id).with_id(id).with_columns(
                    server_id,
                    description,
                    checkbox(enabled),
                )
            })
            .collect(),
        nls::text(language, Message::ConfigNoLanguageServers),
    )
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
