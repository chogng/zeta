use std::collections::BTreeMap;

use zeta_app_server_protocol::protocol::connectors::ConnectorAvailableActionDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorConnectionStateDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorListResult;

use crate::components::list_selection::ListSelectionActivationMode;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConnectorSelectionAction {
    ConnectDeviceOAuth {
        connector_id: String,
        connection_generation: u64,
    },
    Disconnect {
        connector_id: String,
    },
}

pub(crate) struct ConnectorPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, ConnectorSelectionAction>,
}

pub(crate) fn connector_pane_spec(catalog: &ConnectorListResult) -> ConnectorPaneSpec {
    let mut actions = BTreeMap::new();
    let all = catalog
        .connectors
        .iter()
        .enumerate()
        .map(|(index, connector)| connector_item(index, connector, &mut actions))
        .collect::<Vec<_>>();
    let connected = all
        .iter()
        .zip(&catalog.connectors)
        .filter(|(_, connector)| {
            matches!(
                connector.state,
                ConnectorConnectionStateDto::Connected { .. }
            )
        })
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();
    let disconnected = all
        .iter()
        .zip(&catalog.connectors)
        .filter(|(_, connector)| {
            !matches!(
                connector.state,
                ConnectorConnectionStateDto::Connected { .. }
            )
        })
        .map(|(item, _)| item.clone())
        .collect::<Vec<_>>();
    ConnectorPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Connectors",
                vec![
                    ListSelectionGroup::new(format!("All ({})", all.len()), all),
                    ListSelectionGroup::new(format!("Connected ({})", connected.len()), connected),
                    ListSelectionGroup::new(
                        format!("Not connected ({})", disconnected.len()),
                        disconnected,
                    ),
                ],
            )
            .with_activation_mode(ListSelectionActivationMode::Enter)
            .with_search(SearchBoxModel::new("Search connectors"))
            .with_empty_message("No matching Connectors"),
            "↑/↓ focus  ·  ←/→ or Tab/Shift-Tab tabs  ·  Enter connect/disconnect  ·  Esc back",
        ),
        actions,
    }
}

fn connector_item(
    index: usize,
    connector: &ConnectorDto,
    actions: &mut BTreeMap<ListSelectionItemId, ConnectorSelectionAction>,
) -> ListSelectionItem {
    let item_id = ListSelectionItemId::new(format!("connector-{index}"));
    if connector
        .available_actions
        .contains(&ConnectorAvailableActionDto::Disconnect)
    {
        actions.insert(
            item_id.clone(),
            ConnectorSelectionAction::Disconnect {
                connector_id: connector.id.clone(),
            },
        );
    } else if connector.available_actions.iter().any(|action| {
        matches!(
            action,
            ConnectorAvailableActionDto::ConnectOAuth
                | ConnectorAvailableActionDto::ReauthorizeOAuth
        )
    }) && connector
        .oauth_methods
        .contains(&zeta_app_server_protocol::protocol::connectors::ConnectorOAuthMethodDto::Device)
    {
        actions.insert(
            item_id.clone(),
            ConnectorSelectionAction::ConnectDeviceOAuth {
                connector_id: connector.id.clone(),
                connection_generation: connector.connection_generation + 1,
            },
        );
    }
    ListSelectionItem::new(&connector.display_name)
        .with_id(item_id)
        .with_description(format!(
            "{}  ·  {}",
            connector.description,
            state_label(&connector.state)
        ))
}

fn state_label(state: &ConnectorConnectionStateDto) -> String {
    match state {
        ConnectorConnectionStateDto::Disconnected => "not connected".into(),
        ConnectorConnectionStateDto::Connecting => "connecting".into(),
        ConnectorConnectionStateDto::Connected { account } => {
            format!("connected as {}", account.display_name)
        }
        ConnectorConnectionStateDto::Unavailable { reason } => format!("unavailable: {reason}"),
        ConnectorConnectionStateDto::ReauthorizationRequired { account, .. } => {
            format!("reconnect {}", account.display_name)
        }
    }
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
