use std::collections::BTreeMap;

use zeta_app_server_protocol::protocol::connectors::ConnectorAvailableActionDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorConnectionStateDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorListResult;

use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ConnectorSelectionAction {
    Disconnect { connector_id: String },
}

pub(crate) struct ConnectorSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, ConnectorSelectionAction>,
}

pub(crate) fn connector_selection_view(catalog: &ConnectorListResult) -> ConnectorSelectionView {
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
    ConnectorSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Connectors",
                vec![
                    SelectionTab::new(format!("All ({})", all.len()), all),
                    SelectionTab::new(format!("Connected ({})", connected.len()), connected),
                    SelectionTab::new(
                        format!("Not connected ({})", disconnected.len()),
                        disconnected,
                    ),
                ],
            )
            .with_activation_mode(SelectionActivationMode::Enter)
            .with_search(SearchBoxModel::new("Search connectors"))
            .with_empty_message("No matching Connectors"),
            "Space search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Enter disconnect connected account  ·  Esc back",
        ),
        actions,
    }
}

fn connector_item(
    index: usize,
    connector: &ConnectorDto,
    actions: &mut BTreeMap<SelectionItemId, ConnectorSelectionAction>,
) -> SelectionItem {
    let item_id = SelectionItemId::new(format!("connector-{index}"));
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
    }
    SelectionItem::new(&connector.display_name)
        .with_id(item_id)
        .with_description(format!(
            "{}  ·  {}",
            connector.description,
            state_label(&connector.state)
        ))
}

fn state_label(state: &ConnectorConnectionStateDto) -> String {
    match state {
        ConnectorConnectionStateDto::Disconnected => {
            "not connected · connect in Desktop Settings".into()
        }
        ConnectorConnectionStateDto::Connecting => "connecting".into(),
        ConnectorConnectionStateDto::Connected { account } => {
            format!("connected as {}", account.display_name)
        }
        ConnectorConnectionStateDto::Unavailable { reason } => format!("unavailable: {reason}"),
        ConnectorConnectionStateDto::ReauthorizationRequired { account, .. } => {
            format!("reconnect {} in Desktop Settings", account.display_name)
        }
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
