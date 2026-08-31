use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_protocol::Patch;

use super::StatusLineItem;
use super::StatusLineChoices;
use super::StatusLineSettings;
use super::setup::list_selection;
use crate::client::new_command_id;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StatusLineEdit {
    pub(crate) expected_revision: u64,
    pub(crate) item: StatusLineItem,
    pub(crate) enabled: bool,
}

pub(crate) struct StatusLineRegionUpdate {
    pub(crate) settings: StatusLineSettings,
    pub(crate) region_spec: StatusLineChoices,
}

pub(crate) fn read_status_line<T>(
    client: &mut AppServerClient<T>,
) -> Result<StatusLineRegionUpdate, String>
where
    T: JsonRpcTransport,
{
    let config = client.read_config().map_err(|error| error.to_string())?;
    let settings = StatusLineSettings::from_tui(&config.tui)?;
    let region_spec = list_selection(&settings, config.revision);
    Ok(StatusLineRegionUpdate {
        settings,
        region_spec,
    })
}

pub(crate) fn set_status_line<T>(
    client: &mut AppServerClient<T>,
    edit: StatusLineEdit,
) -> Result<StatusLineRegionUpdate, String>
where
    T: JsonRpcTransport,
{
    let config = client.read_config().map_err(|error| error.to_string())?;
    if config.revision != edit.expected_revision {
        return Err(
            "configuration changed after the status-line editor opened; reopen /statusline and try again"
                .into(),
        );
    }
    let mut settings = StatusLineSettings::from_tui(&config.tui)?;
    settings.set(edit.item, edit.enabled);
    let tui = settings.write_to_tui(&config.tui);
    client
        .update_config(ConfigUpdateParams {
            command_id: new_command_id("status-line"),
            expected_revision: config.revision,
            preferred_model: Patch::Missing,
            approval_review_model: Patch::Missing,
            commit_message_model: Patch::Missing,
            tool_mode: Patch::Missing,
            agent_grep_backend: Patch::Missing,
            gui: Patch::Missing,
            tui: Patch::Value(tui),
        })
        .map_err(|error| error.to_string())?;

    let config = client.read_config().map_err(|error| error.to_string())?;
    let settings = StatusLineSettings::from_tui(&config.tui)?;
    let region_spec = list_selection(&settings, config.revision);
    Ok(StatusLineRegionUpdate {
        settings,
        region_spec,
    })
}
