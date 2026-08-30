mod pane;
mod request;
mod resource;
mod settings;
mod settings_migration;

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
pub(crate) use request::set_tui_theme;
pub(crate) use resource::ConfigResource;
pub(crate) use resource::TerminalSettingsEdit;
pub(crate) use settings::FollowUpMode;
pub(crate) use settings::TerminalSettings;

pub(crate) struct TerminalSettingsSnapshot {
    pub(crate) settings: TerminalSettings,
    pub(crate) revision: u64,
}

pub(crate) struct ConfigEditResult {
    pub(crate) settings: TerminalSettings,
    pub(crate) pane_spec: ConfigPaneSpec,
}

pub(crate) fn refresh_terminal_settings(
    resource: Option<&mut ConfigResource>,
) -> Result<TerminalSettingsSnapshot, String> {
    match resource {
        Some(resource) => {
            let settings = resource.refresh()?;
            Ok(TerminalSettingsSnapshot {
                settings,
                revision: resource.revision(),
            })
        }
        None => Ok(TerminalSettingsSnapshot {
            settings: TerminalSettings::default(),
            revision: 0,
        }),
    }
}

pub(crate) fn apply_config_edit(
    resource: Option<&mut ConfigResource>,
    edit: &ConfigEdit,
) -> Result<ConfigEditResult, String> {
    let resource = resource.ok_or_else(|| {
        "terminal settings are unavailable because no active profile root was configured".to_owned()
    })?;
    let (settings, revision) = resource.apply_edit(&edit.terminal)?;
    Ok(ConfigEditResult {
        settings,
        pane_spec: config_pane_spec(
            &edit.server_config,
            &edit.providers,
            settings,
            revision,
            &edit.session_id,
            &edit.dirs,
        ),
    })
}
