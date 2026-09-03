mod editor;
mod request;
mod settings;

pub(crate) use editor::ConfigChoices;
pub(crate) use editor::ConfigEdit;
pub(crate) use editor::ConfigEditor;
pub(crate) use editor::ConfigEditorOutcome;
pub(crate) use editor::ConfigSelectionAction;
pub(crate) use editor::ProviderApiKeyEdit;
pub(crate) use editor::config_choices;
pub(crate) use request::read_config_choices;
pub(crate) use request::set_provider_api_key;
pub(crate) use request::set_terminal_settings;
pub(crate) use settings::FollowUpMode;
pub(crate) use settings::TerminalSettings;

pub(crate) struct ConfigEditResult {
    pub(crate) settings: TerminalSettings,
    pub(crate) choices: ConfigChoices,
}
