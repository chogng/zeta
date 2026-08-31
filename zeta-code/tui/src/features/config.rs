mod editor;
mod request;
mod settings;

pub(crate) use editor::ConfigChoices;
pub(crate) use editor::ConfigEdit;
pub(crate) use editor::ConfigEditor;
pub(crate) use editor::ConfigEditorOutcome;
pub(crate) use editor::ConfigSelectionAction;
pub(crate) use editor::PermissionEdit;
pub(crate) use editor::ProviderApiKeyEdit;
pub(crate) use editor::config_choices;
pub(crate) use request::PreferredModelUpdate;
pub(crate) use request::preferred_model;
pub(crate) use request::read_config_choices;
pub(crate) use request::set_permissions;
pub(crate) use request::set_preferred_model;
pub(crate) use request::set_provider_api_key;
pub(crate) use request::set_terminal_settings;
pub(crate) use request::set_tui_theme;
pub(crate) use request::tui_theme;
pub(crate) use settings::FollowUpMode;
pub(crate) use settings::TerminalSettings;

pub(crate) struct ConfigEditResult {
    pub(crate) settings: TerminalSettings,
    pub(crate) choices: ConfigChoices,
}
