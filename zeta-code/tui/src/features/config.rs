mod region;
mod request;
mod settings;

pub(crate) use region::ConfigEdit;
pub(crate) use region::ConfigEditor;
pub(crate) use region::ConfigEditorOutcome;
pub(crate) use region::ConfigChoices;
pub(crate) use region::ConfigSelectionAction;
pub(crate) use region::PermissionEdit;
pub(crate) use region::ProviderApiKeyEdit;
pub(crate) use region::config_choices;
pub(crate) use request::PreferredModelUpdate;
pub(crate) use request::preferred_model;
pub(crate) use request::read_config_region;
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
    pub(crate) region_spec: ConfigChoices,
}
