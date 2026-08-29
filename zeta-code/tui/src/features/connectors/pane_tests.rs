use zeta_app_server_protocol::protocol::connectors::ConnectorAvailableActionDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorConnectionStateDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorDto;
use zeta_app_server_protocol::protocol::connectors::ConnectorListResult;
use zeta_app_server_protocol::protocol::connectors::ConnectorOAuthMethodDto;

use super::ConnectorSelectionAction;
use super::connector_pane_spec;
use crate::components::list_selection::ListSelectionState;

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

    let view = connector_pane_spec(&catalog);
    let state = ListSelectionState::new(view.model.into_body());

    assert_eq!(state.title(), "Connectors");
    assert_eq!(view.actions.len(), 1);
    assert!(matches!(
        view.actions.values().next(),
        Some(ConnectorSelectionAction::Disconnect { connector_id }) if connector_id == "github"
    ));
}

#[test]
fn disconnected_device_oauth_connector_is_actionable() {
    let mut github = connector(
        "github",
        ConnectorConnectionStateDto::Disconnected,
        vec![ConnectorAvailableActionDto::ConnectOAuth],
    );
    github.oauth_methods = vec![ConnectorOAuthMethodDto::Device];
    let view = connector_pane_spec(&ConnectorListResult {
        generation: 4,
        connectors: vec![github],
    });

    assert!(matches!(
        view.actions.values().next(),
        Some(ConnectorSelectionAction::ConnectDeviceOAuth {
            connector_id,
            connection_generation: 2,
        }) if connector_id == "github"
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
        oauth_methods: Vec::new(),
        credential_cleanup_pending: false,
    }
}
