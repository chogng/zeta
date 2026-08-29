use super::McpSelectionAction;
use super::mcp_pane_spec;
use crate::components::list_selection::ListSelectionState;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::McpCredentialBindingDto;
use zeta_app_server_protocol::protocol::config::McpServerConfigDto;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::config::McpTransportDto;

#[test]
fn mcp_pane_filters_servers_and_maps_enter_to_the_opposite_enablement() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "docs".into(),
        McpServerConfigDto {
            id: "docs".into(),
            display_name: "Documentation".into(),
            transport: McpTransportDto::Stdio {
                command: "docs-server".into(),
                args: vec!["--stdio".into()],
            },
            credential: McpCredentialBindingDto::Unauthenticated,
            enablement: McpServerEnablementDto::Enabled,
        },
    );

    let view = mcp_pane_spec(&servers);
    let state = ListSelectionState::new(view.model.into_body());

    assert_eq!(state.title(), "MCP servers");
    assert_eq!(state.tabs()[0].label(), "All (1)");
    assert_eq!(state.visible_items()[0].label(), "Documentation");
    assert_eq!(
        view.actions.values().next(),
        Some(&McpSelectionAction::SetEnablement {
            server_id: "docs".into(),
            enablement: McpServerEnablementDto::Disabled,
        })
    );
}
