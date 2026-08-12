use super::config_view;
use crate::components::selection::SelectionViewState;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;

#[test]
fn config_pane_organizes_the_snapshot_into_searchable_tabs() {
    let state = SelectionViewState::new(config_view(&empty_config()).into_body());

    assert_eq!(state.title(), "Config");
    assert!(state.search().is_some());
    assert_eq!(
        state
            .tabs()
            .iter()
            .map(|tab| tab.label())
            .collect::<Vec<_>>(),
        vec![
            "Overview",
            "Providers",
            "MCP",
            "Skill sources",
            "Plugins",
            "Hooks",
            "Language servers",
        ]
    );
    assert_eq!(state.visible_items()[0].label(), "Revision");
    assert_eq!(state.visible_items()[0].description(), Some("4"));
}

fn empty_config() -> ConfigReadResult {
    ConfigReadResult {
        revision: 4,
        generation: 5,
        preferred_model: None,
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        providers: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
        skill_sources: BTreeMap::new(),
        plugin_requests: BTreeMap::new(),
        hooks: BTreeMap::new(),
        language_servers: BTreeMap::new(),
        tool_search: zeta_app_server_protocol::protocol::config::ToolSearchConfigDto {
            mode: zeta_app_server_protocol::protocol::config::ToolSearchModeDto::Lexical,
            embedding_model: None,
            embedding_status: zeta_app_server_protocol::protocol::config::ToolSearchEmbeddingStatusDto::Disabled,
        },
        semantic_code_index: zeta_app_server_protocol::protocol::config::SemanticCodeIndexConfigDto {
            selection: zeta_app_server_protocol::protocol::config::SemanticCodeIndexSelectionDto::Disabled,
            automatic_context: zeta_app_server_protocol::protocol::config::SemanticCodeIndexAutomaticContextDto::Off,
            active_workspace_authorized: false,
        },
    }
}
