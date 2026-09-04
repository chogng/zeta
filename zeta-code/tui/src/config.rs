mod editor;
mod request;
mod settings;

pub(crate) use editor::ConfigChoices;
pub(crate) use editor::ConfigEdit;
pub(crate) use editor::ConfigEditor;
pub(crate) use editor::ConfigEditorOutcome;
pub(crate) use editor::ConfigEditorPage;
pub(crate) use editor::ConfigSelectionAction;
pub(crate) use editor::LanguageServerEdit;
pub(crate) use editor::ProviderApiKeyEdit;
pub(crate) use editor::config_choices;
pub(crate) use request::execute;
#[cfg(test)]
pub(crate) use request::set_settings;
pub(crate) use settings::TerminalSettings;

pub(crate) struct ConfigEditResult {
    pub(crate) terminal: TerminalSettings,
    pub(crate) status_line: crate::status::StatusLineSettings,
    pub(crate) choices: ConfigChoices,
}

/// A completed configuration operation delivered to the TUI state owner.
pub(crate) enum Event {
    SettingsReceived(TerminalSettings),
    Updated(ConfigEditResult),
    EditorOpened(ConfigChoices),
    ApiKeySaved {
        provider: String,
        choices: ConfigChoices,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    OpenEditor,
    Edit(ConfigEdit),
    SetLanguageServerMode(LanguageServerEdit),
    SetProviderApiKey(ProviderApiKeyEdit),
}
