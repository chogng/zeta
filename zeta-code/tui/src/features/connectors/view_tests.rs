use zeta_app_server_protocol::protocol::connectors::ConnectorAvailableActionDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorConnectionStateDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorListResult;

use super::ConnectorSelectionAction;
use super::connector_selection_view;
use crate::components::selection::SelectionViewState;

#[test]
fn connected_connector_is_actionable_while_disconnected_connector_is_read_only() {
    let catalog = ConnectorListResult {
        generation: 3,
        connectors: vec![
            connector(
                "github",
                ConnectorConnectionStateDto::Connected {
                    account: zeta_app_server_protocol::protocol::connectors::ConnectorAccountDto {
                        id: "octocat".into(),
                        display_name: "Octocat".into(),
                    },
                },
                vec![ConnectorAvailableActionDto::Disconnect],
            ),
            connector(
                "slack",
                ConnectorConnectionStateDto::Disconnected,
                vec![ConnectorAvailableActionDto::ConnectApiToken],
            ),
        ],
    };

    let view = connector_selection_view(&catalog);
    let state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.title(), "Connectors");
    assert_eq!(view.actions.len(), 1);
    assert!(matches!(
        view.actions.values().next(),
        Some(ConnectorSelectionAction::Disconnect { connector_id }) if connector_id == "github"
    ));
}

fn connector(
    id: &str,
    state: ConnectorConnectionStateDto,
    available_actions: Vec<ConnectorAvailableActionDto>,
) -> ConnectorDto {
    ConnectorDto {
        id: id.into(),
        display_name: id.into(),
        description: format!("Connect {id}."),
        runtime_server_id: format!("plugin:acme/{id}:mcp:{id}"),
        definition_digest: "sha256:definition".into(),
        connection_generation: 1,
        state,
        available_actions,
        credential_cleanup_pending: false,
    }
}
