use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::AgentGrepBackendDto;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::SemanticCodeIndexAutomaticContextDto;
use zeta_app_server_protocol::protocol::config::SemanticCodeIndexConfigDto;
use zeta_app_server_protocol::protocol::config::SemanticCodeIndexSelectionDto;
use zeta_app_server_protocol::protocol::config::ToolSearchConfigDto;
use zeta_app_server_protocol::protocol::config::ToolSearchEmbeddingStatusDto;
use zeta_app_server_protocol::protocol::config::ToolSearchModeDto;

pub(crate) fn empty_config_snapshot() -> ConfigReadResult {
    ConfigReadResult {
        revision: 0,
        generation: 0,
        preferred_model: None,
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        tool_mode: zeta_protocol::ToolMode::Direct,
        agent_grep_backend: AgentGrepBackendDto::Ripgrep,
        providers: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
        skill_sources: BTreeMap::new(),
        plugin_requests: BTreeMap::new(),
        hooks: BTreeMap::new(),
        language_servers: BTreeMap::new(),
        tool_search: ToolSearchConfigDto {
            mode: ToolSearchModeDto::Lexical,
            embedding_model: None,
            embedding_status: ToolSearchEmbeddingStatusDto::Disabled,
        },
        semantic_code_index: SemanticCodeIndexConfigDto {
            selection: SemanticCodeIndexSelectionDto::Disabled,
            automatic_context: SemanticCodeIndexAutomaticContextDto::Off,
            active_workspace_authorized: false,
        },
        exec_policy_rules: Vec::new(),
    }
}
