mod pane;
mod request;
mod settings;

pub(crate) use pane::ConfigEdit;
pub(crate) use pane::ConfigPaneSpec;
pub(crate) use pane::ConfigSelectionAction;
pub(crate) use pane::PermissionEdit;
pub(crate) use pane::ProviderApiKeyEdit;
pub(crate) use pane::config_pane_spec;
pub(crate) use pane::provider_api_key_prompt;
pub(crate) use request::PreferredModelUpdate;
pub(crate) use request::preferred_model;
pub(crate) use request::read_config_pane;
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
    pub(crate) pane_spec: ConfigPaneSpec,
}
