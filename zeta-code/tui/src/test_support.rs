use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::Mutex;
use std::sync::MutexGuard;
use zeta_app_server_protocol::protocol::config::AgentGrepBackendDto;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::CodebaseAutomaticContextDto;
use zeta_app_server_protocol::protocol::config::CodebaseConfigDto;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;
use zeta_app_server_protocol::protocol::config::ToolSearchConfigDto;
use zeta_app_server_protocol::protocol::config::ToolSearchEmbeddingStatusDto;
use zeta_app_server_protocol::protocol::config::ToolSearchModeDto;

static IN_PROCESS_TEST_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn snapshot_name(name: impl Display, module_path: &str) -> String {
    let module_path = module_path
        .split_once("::")
        .map_or(module_path, |(_, module_path)| module_path)
        .replace("::", "__");
    let name = name.to_string().replace(['/', '\\'], "__");
    format!("{module_path}__{name}")
}

pub(crate) fn snapshot_name_for_function(function_path: &str, module_path: &str) -> String {
    let function_name = function_path.rsplit("::").next().unwrap_or(function_path);
    let function_name = function_name.strip_prefix("test_").unwrap_or(function_name);
    snapshot_name(function_name, module_path)
}

pub(crate) fn in_process_test_guard() -> MutexGuard<'static, ()> {
    IN_PROCESS_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn empty_config_snapshot() -> ConfigReadResult {
    ConfigReadResult {
        time_context: Default::default(),
        features: vec![],
        revision: 0,
        generation: 0,
        preferred_model: None,
        preferred_reasoning_effort: None,
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        commit_message_model: None,
        commit_message_active_dir_authorized: false,
        issues: zeta_app_server_protocol::protocol::issues::IssueConfigDto {
            auto_refresh_minutes: 10,
        },
        tool_mode: zeta_protocol::ToolMode::Direct,
        agent_grep_backend: AgentGrepBackendDto::Ripgrep,
        gui: FrontendConfigDto::default(),
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
        tui: FrontendConfigDto::default(),
    }
}

/// Scripted server acceptance for a text-only queue submission in App simulations.
pub(crate) fn queued_message(command: crate::app::AppCommand) -> ::queue::QueuedMessage {
    let crate::app::AppCommand::Thread(crate::thread::Command::Enqueue {
        command_id,
        submission,
        ..
    }) = command
    else {
        panic!("expected queue enqueue command")
    };
    let input = submission
        .input
        .into_iter()
        .map(|input| match input {
            crate::thread::composer::ChatInputItem::Context { name, content } => {
                zeta_protocol::UserInput::Context { name, content }
            }
            crate::thread::composer::ChatInputItem::Text(text) => {
                zeta_protocol::UserInput::Text { text }
            }
            crate::thread::composer::ChatInputItem::Attachment(attachment) => {
                zeta_protocol::UserInput::ImageAttachment { attachment }
            }
            crate::thread::composer::ChatInputItem::Skill { skill } => {
                zeta_protocol::UserInput::Skill { skill }
            }
            crate::thread::composer::ChatInputItem::Image { .. } => {
                panic!("image tests must provide a materialized server attachment")
            }
        })
        .collect();
    ::queue::QueuedMessage {
        request: ::queue::QueueInput {
            command_id,
            session_id: zeta_protocol::SessionId::new("session").unwrap(),
            thread_id: zeta_protocol::ThreadId::new("thread").unwrap(),
            directory: "/work".into(),
            input,
            tool_mode: Default::default(),
            approval_mode: Default::default(),
            steer_turn: None,
        },
        status: ::queue::QueueStatus::Pending,
        turn_id: None,
        error: None,
        revision: 1,
    }
}
