use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::MutexGuard;
use zeta_app_server_protocol::protocol::config::AgentGrepBackendDto;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::CodebaseAutomaticContextDto;
use zeta_app_server_protocol::protocol::config::CodebaseConfigDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::ToolSearchConfigDto;
use zeta_app_server_protocol::protocol::config::ToolSearchEmbeddingStatusDto;
use zeta_app_server_protocol::protocol::config::ToolSearchModeDto;
use zeta_app_server_protocol::protocol::config::TuiConfigDto;

static IN_PROCESS_TEST_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn in_process_test_guard() -> MutexGuard<'static, ()> {
    IN_PROCESS_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn empty_config_snapshot() -> ConfigReadResult {
    ConfigReadResult {
        revision: 0,
        generation: 0,
        preferred_model: None,
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        commit_message_model: None,
        commit_message_active_dir_authorized: false,
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
        codebase: CodebaseConfigDto {
            models: None,
            automatic_context: CodebaseAutomaticContextDto::Off,
        },
        exec_policy_rules: Vec::new(),
        tui: TuiConfigDto {
            theme: "system".into(),
        },
    }
}
