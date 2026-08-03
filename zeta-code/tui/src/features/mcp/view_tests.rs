use super::McpSelectionAction;
use super::mcp_selection_view;
use crate::components::selection::SelectionViewState;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::McpCredentialBindingDto;
use zeta_app_server_protocol::protocol::config::McpServerConfigDto;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::config::McpTransportDto;

#[test]
fn mcp_pane_filters_servers_and_maps_enter_to_the_opposite_enablement() {
    let mut config = empty_config();
    config.mcp_servers.insert(
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

    let view = mcp_selection_view(&config);
    let state = SelectionViewState::new(view.model.into_body());

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

fn empty_config() -> ConfigReadResult {
    ConfigReadResult {
        revision: 0,
        generation: 0,
        preferred_model: None,
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        providers: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
        skill_sources: BTreeMap::new(),
        plugin_requests: BTreeMap::new(),
        hooks: BTreeMap::new(),
        language_servers: BTreeMap::new(),
    }
}
