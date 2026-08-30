use super::AppServer;
use super::RpcError;
use super::decode;
use super::extension_config_operations::hook_config_dto;
use super::extension_config_operations::plugin_request_dto;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::codebase::FastRegexDisableAndDeleteParams;
use zeta_app_server_protocol::protocol::codebase::FastRegexDisableAndDeleteResult;
use zeta_app_server_protocol::protocol::codebase::LocalIndexClearOutcomeDto;
use zeta_app_server_protocol::protocol::config::AgentGrepBackendDto;
use zeta_app_server_protocol::protocol::config::ApprovalReviewModelSelectionDto;
use zeta_app_server_protocol::protocol::config::CodebaseAutomaticContextDto;
use zeta_app_server_protocol::protocol::config::CodebaseConfigDto;
use zeta_app_server_protocol::protocol::config::CodebaseConfigureParams;
use zeta_app_server_protocol::protocol::config::CodebaseModelsDto;
use zeta_app_server_protocol::protocol::config::CommitMessageAuthorizeParams;
use zeta_app_server_protocol::protocol::config::CommitMessageRevokeParams;
use zeta_app_server_protocol::protocol::config::ConfigCommandDispositionDto;
use zeta_app_server_protocol::protocol::config::ConfigCommandResult;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::ConfigUpdateParams;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigDto;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigureParams;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::config::LanguageServerRemoveParams;
use zeta_app_server_protocol::protocol::config::McpCredentialBindingDto;
use zeta_app_server_protocol::protocol::config::McpServerConfigDto;
use zeta_app_server_protocol::protocol::config::McpServerEnablementDto;
use zeta_app_server_protocol::protocol::config::McpServerRemoveParams;
use zeta_app_server_protocol::protocol::config::McpServerSetEnablementParams;
use zeta_app_server_protocol::protocol::config::McpServerUpsertParams;
use zeta_app_server_protocol::protocol::config::McpTransportDto;
use zeta_app_server_protocol::protocol::config::ModelContextConfigDto;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::config::ProviderConfigDto;
use zeta_app_server_protocol::protocol::config::ProviderConfigureParams;
use zeta_app_server_protocol::protocol::config::ProviderRemoveParams;
use zeta_app_server_protocol::protocol::config::SkillSourceAddParams;
use zeta_app_server_protocol::protocol::config::SkillSourceConfigDto;
use zeta_app_server_protocol::protocol::config::SkillSourceEnablementDto;
use zeta_app_server_protocol::protocol::config::SkillSourceRemoveParams;
use zeta_app_server_protocol::protocol::config::SkillSourceSetEnablementParams;
use zeta_app_server_protocol::protocol::config::ToolSearchConfigDto;
use zeta_app_server_protocol::protocol::config::ToolSearchConfigureParams;
use zeta_app_server_protocol::protocol::config::ToolSearchEmbeddingStatusDto;
use zeta_app_server_protocol::protocol::config::ToolSearchModeDto;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_config::AgentGrepBackend;
use zeta_config::ApprovalReviewModelSelection;
use zeta_config::CodebaseAutomaticContext;
use zeta_config::CodebaseModelSelection;
use zeta_config::ConfigCommandDisposition;
use zeta_config::ConfigCommandError;
use zeta_config::ConfigCommandRequest;
use zeta_config::ConfigRevision;
use zeta_config::LanguageServerConfig;
use zeta_config::LanguageServerId;
use zeta_config::LanguageServerModeConfig;
use zeta_config::McpCredentialBinding;
use zeta_config::McpServerConfig;
use zeta_config::McpServerEnablement;
use zeta_config::McpServerId;
use zeta_config::McpTransportConfig;
use zeta_config::PreferencesUpdate;
use zeta_config::ResolvedConfigSnapshot;
use zeta_config::SkillSourceConfig;
use zeta_config::SkillSourceEnablement;
use zeta_config::SkillSourceId;
use zeta_config::ToolSearchConfig;
use zeta_config::ToolSearchModeConfig;
use zeta_config::UserConfigCommand;
use zeta_model_provider::ModelId;
use zeta_model_provider::ModelRef;
use zeta_model_provider::ProviderId;
use zeta_model_provider_config::ModelContextConfig;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::Patch;
use zeta_state::ClearOutcome;
use zeta_state::DirIndexKind;

use crate::tool_search_models::ToolSearchEmbeddingStatus;
use crate::tool_search_models::resolve_tool_search;
use zeta_app_server_protocol::protocol::config::ExecPolicyActionKindDto;
use zeta_app_server_protocol::protocol::config::ExecPolicyEffectDto;
use zeta_app_server_protocol::protocol::config::ExecPolicyHostMatcherDto;
use zeta_app_server_protocol::protocol::config::ExecPolicyRuleDto;
use zeta_app_server_protocol::protocol::config::ExecPolicyRuleRemoveParams;
use zeta_app_server_protocol::protocol::config::ExecPolicyRuleUpsertParams;
use zeta_app_server_protocol::protocol::config::ExecPolicyScopeMatcherDto;
use zeta_app_server_protocol::protocol::config::ExecPolicySelectorDto;
use zeta_app_server_protocol::protocol::config::ExecPolicyTokenDto;
use zeta_execpolicy::ExecPolicyActionKind;
use zeta_execpolicy::ExecPolicyEffect;
use zeta_execpolicy::ExecPolicyRule;
use zeta_execpolicy::ExecPolicyRuleId;
use zeta_execpolicy::ExecPolicySelector;
use zeta_execpolicy::ExecPolicyToken;
use zeta_execpolicy::HostMatcher;
use zeta_execpolicy::ScopeMatcher;

impl AppServer {
    pub(super) fn config_read(&self) -> Result<Value, RpcError> {
        let snapshot = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
            .read_snapshot()
            .map_err(config_error)?;
        result(&config_read_result(
            snapshot,
            self.active_dir_id().as_ref(),
            self.tool_search_embedding_status(),
        ))
    }

    pub(super) fn tool_search_configure(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ToolSearchConfigureParams = decode(params)?;
        let config = tool_search_config_from_dto(params.mode, params.embedding_model)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let snapshot = store.read_snapshot().map_err(config_error)?;
        let resolution = resolve_tool_search(
            &config,
            &snapshot.values.providers,
            self.semantic_model_provider.as_ref(),
        );
        if let ToolSearchEmbeddingStatus::Unavailable { reason, .. } = resolution.status {
            log::warn!("Tool Search embedding readiness probe rejected configuration: {reason}");
            return Err(RpcError::new(
                -32092,
                AppServerErrorName::ToolSearchUnavailable,
            ));
        }
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::ConfigureToolSearch { config },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn config_update(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ConfigUpdateParams = decode(params)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
                    preferred_model: model_ref_update_from_dto(params.preferred_model)?,
                    approval_review_model: approval_review_model_update_from_dto(
                        params.approval_review_model,
                    )?,
                    commit_message_model: model_ref_update_from_dto(params.commit_message_model)?,
                    tool_mode: params.tool_mode,
                    grep_backend: params.agent_grep_backend.map(agent_grep_backend_from_dto),
                }),
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn fast_regex_disable_and_delete(&self, params: &Value) -> Result<Value, RpcError> {
        let params: FastRegexDisableAndDeleteParams = decode(params)?;
        let dir = self
            .active_dir_id()
            .ok_or_else(|| RpcError::new(-32090, AppServerErrorName::CodebaseUnavailable))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
                    preferred_model: Patch::Missing,
                    approval_review_model: Patch::Missing,
                    commit_message_model: Patch::Missing,
                    tool_mode: Patch::Missing,
                    grep_backend: Patch::Value(AgentGrepBackend::Ripgrep),
                }),
            })
            .map_err(config_operation_error)?;
        let snapshot = store.read_snapshot().map_err(config_error)?;
        self.env_runtime_control()
            .ok_or_else(|| RpcError::new(-32090, AppServerErrorName::CodebaseUnavailable))?
            .reconcile_local_tool_config(&snapshot.values)
            .map_err(|_| RpcError::new(-32092, AppServerErrorName::CodebaseOperationFailed))?;
        let deletion = match &self.env_state {
            super::EnvStateMode::Persistent(state) => state
                .clear_index(&dir, DirIndexKind::AgentGrep)
                .map_err(|_| RpcError::new(-32092, AppServerErrorName::CodebaseOperationFailed))?,
            super::EnvStateMode::Ephemeral => ClearOutcome::AlreadyAbsent,
            super::EnvStateMode::Unconfigured => {
                return Err(RpcError::new(
                    -32090,
                    AppServerErrorName::CodebaseUnavailable,
                ));
            }
        };
        result(&FastRegexDisableAndDeleteResult {
            config: config_command_result(outcome),
            deletion: match deletion {
                ClearOutcome::Cleared => LocalIndexClearOutcomeDto::Cleared,
                ClearOutcome::AlreadyAbsent => LocalIndexClearOutcomeDto::AlreadyAbsent,
                ClearOutcome::InUse => LocalIndexClearOutcomeDto::InUse,
            },
        })
    }

    pub(super) fn exec_policy_rule_upsert(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ExecPolicyRuleUpsertParams = decode(params)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::UpsertExecPolicyRule {
                    rule: exec_policy_rule_from_dto(params.rule),
                },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn exec_policy_rule_remove(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ExecPolicyRuleRemoveParams = decode(params)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::RemoveExecPolicyRule {
                    rule_id: ExecPolicyRuleId::new(params.rule_id),
                },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn codebase_configure(&self, params: &Value) -> Result<Value, RpcError> {
        let params: CodebaseConfigureParams = decode(params)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::ConfigureCodebase {
                    models: params.models.map(codebase_models_from_dto).transpose()?,
                    automatic_context: semantic_automatic_context_from_dto(
                        params.automatic_context,
                    ),
                },
            })
            .map_err(config_operation_error)?;
        if let Some(semantic) = self.codebase_semantic_service()
            && let Err(error) = semantic.delete_index()
        {
            log::warn!("failed to clear replaced semantic codebase projection: {error}");
        }
        self.reconcile_codebase_runtime()
            .map_err(|_| RpcError::new(-32092, AppServerErrorName::CodebaseOperationFailed))?;
        result(&config_command_result(outcome))
    }

    pub(super) fn commit_message_authorize(&self, params: &Value) -> Result<Value, RpcError> {
        let params: CommitMessageAuthorizeParams = decode(params)?;
        let dir = self
            .active_dir_id()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::AuthorizeCommitMessageEgress { dir },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn commit_message_revoke(&self, params: &Value) -> Result<Value, RpcError> {
        let params: CommitMessageRevokeParams = decode(params)?;
        let dir = self
            .active_dir_id()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::RevokeCommitMessageEgress { dir },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn provider_configure(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ProviderConfigureParams = decode(params)?;
        let provider = provider_config_from_dto(params.config)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::ConfigureProvider {
                    provider: provider.provider.clone(),
                    config: provider,
                },
            })
            .map_err(config_operation_error)?;
        self.reconcile_codebase_runtime()
            .map_err(|_| RpcError::new(-32092, AppServerErrorName::CodebaseOperationFailed))?;
        result(&config_command_result(outcome))
    }

    pub(super) fn language_server_configure(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageServerConfigureParams = decode(params)?;
        let server_id = LanguageServerId::new(params.server_id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let config = language_server_config_from_dto(params.config);
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::ConfigureLanguageServer { server_id, config },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn language_server_remove(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageServerRemoveParams = decode(params)?;
        let server_id = LanguageServerId::new(params.server_id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::RemoveLanguageServerConfiguration { server_id },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn provider_remove(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ProviderRemoveParams = decode(params)?;
        let provider = ProviderId::new(params.provider)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::RemoveProvider { provider },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn mcp_server_upsert(&self, params: &Value) -> Result<Value, RpcError> {
        let params: McpServerUpsertParams = decode(params)?;
        let server = mcp_server_config_from_dto(params.server)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::UpsertMcpServer { server },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn mcp_server_remove(&self, params: &Value) -> Result<Value, RpcError> {
        let params: McpServerRemoveParams = decode(params)?;
        let server_id = McpServerId::new(params.server_id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::RemoveMcpServer { server_id },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn mcp_server_set_enablement(&self, params: &Value) -> Result<Value, RpcError> {
        let params: McpServerSetEnablementParams = decode(params)?;
        let server_id = McpServerId::new(params.server_id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::SetMcpServerEnablement {
                    server_id,
                    enablement: mcp_enablement_from_dto(params.enablement),
                },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn skill_source_add(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SkillSourceAddParams = decode(params)?;
        let source = skill_source_config_from_dto(params.source)?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::AddSkillSource { source },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn skill_source_remove(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SkillSourceRemoveParams = decode(params)?;
        let source_id = SkillSourceId::new(params.source_id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::RemoveSkillSource { source_id },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }

    pub(super) fn skill_source_set_enablement(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SkillSourceSetEnablementParams = decode(params)?;
        let source_id = SkillSourceId::new(params.source_id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::SetSkillSourceEnablement {
                    source_id,
                    enablement: skill_enablement_from_dto(params.enablement),
                },
            })
            .map_err(config_operation_error)?;
        result(&config_command_result(outcome))
    }
}

fn config_error(_: zeta_config::ConfigError) -> RpcError {
    RpcError::new(-32030, AppServerErrorName::ConfigUnavailable)
}

pub(super) fn config_operation_error(error: ConfigCommandError) -> RpcError {
    match error {
        ConfigCommandError::CommandConflict => {
            RpcError::new(-32004, AppServerErrorName::CommandConflict)
        }
        ConfigCommandError::RevisionConflict { .. } => {
            RpcError::new(-32031, AppServerErrorName::ConfigRevisionConflict)
        }
        ConfigCommandError::Config(error) => config_error(error),
    }
}

fn config_read_result(
    snapshot: ResolvedConfigSnapshot,
    active_dir: Option<&zeta_file_access::DirId>,
    tool_search_status: ToolSearchEmbeddingStatus,
) -> ConfigReadResult {
    let commit_message_active_dir_authorized = active_dir.is_some_and(|dir| {
        snapshot
            .values
            .commit_messages
            .authorized_model(
                dir,
                snapshot.values.commit_message_model.as_ref(),
                &snapshot.values.providers,
            )
            .is_some()
    });
    let codebase = CodebaseConfigDto {
        models: snapshot
            .values
            .codebase
            .models
            .clone()
            .map(codebase_models_dto),
        automatic_context: semantic_automatic_context_dto(
            snapshot.values.codebase.automatic_context,
        ),
    };
    let tool_search = ToolSearchConfigDto {
        mode: tool_search_mode_dto(snapshot.values.tool_search.mode),
        embedding_model: snapshot
            .values
            .tool_search
            .embedding_model
            .clone()
            .map(model_ref_dto),
        embedding_status: tool_search_status_dto(tool_search_status),
    };
    ConfigReadResult {
        revision: snapshot.revision.get(),
        generation: snapshot.generation.get(),
        preferred_model: snapshot.values.preferred_model.map(model_ref_dto),
        approval_review_model: approval_review_model_dto(snapshot.values.approval_review_model),
        commit_message_model: snapshot.values.commit_message_model.map(model_ref_dto),
        commit_message_active_dir_authorized,
        tool_mode: snapshot.values.tool_mode,
        agent_grep_backend: agent_grep_backend_dto(snapshot.values.agent_grep_backend),
        providers: snapshot
            .values
            .providers
            .into_iter()
            .map(|(id, config)| (id.to_string(), provider_config_dto(config)))
            .collect(),
        mcp_servers: snapshot
            .values
            .mcp
            .servers
            .into_iter()
            .map(|(id, config)| (id.to_string(), mcp_server_config_dto(config)))
            .collect(),
        skill_sources: snapshot
            .values
            .skills
            .sources
            .into_iter()
            .map(|(id, config)| (id.to_string(), skill_source_config_dto(config)))
            .collect(),
        plugin_requests: snapshot
            .values
            .plugins
            .requests
            .into_iter()
            .map(|(id, request)| (id.to_string(), plugin_request_dto(request)))
            .collect(),
        hooks: snapshot
            .values
            .hooks
            .hooks
            .into_iter()
            .map(|(id, hook)| (id.to_string(), hook_config_dto(hook)))
            .collect(),
        language_servers: snapshot
            .values
            .language_servers
            .servers
            .into_iter()
            .map(|(id, config)| (id.to_string(), language_server_config_dto(config)))
            .collect(),
        tool_search,
        codebase,
        exec_policy_rules: snapshot
            .values
            .exec_policy
            .rules
            .into_iter()
            .map(exec_policy_rule_dto)
            .collect(),
    }
}

fn agent_grep_backend_dto(backend: AgentGrepBackend) -> AgentGrepBackendDto {
    match backend {
        AgentGrepBackend::Ripgrep => AgentGrepBackendDto::Ripgrep,
        AgentGrepBackend::FastRegex => AgentGrepBackendDto::FastRegex,
    }
}

fn agent_grep_backend_from_dto(backend: AgentGrepBackendDto) -> AgentGrepBackend {
    match backend {
        AgentGrepBackendDto::Ripgrep => AgentGrepBackend::Ripgrep,
        AgentGrepBackendDto::FastRegex => AgentGrepBackend::FastRegex,
    }
}

fn exec_policy_rule_dto(rule: ExecPolicyRule) -> ExecPolicyRuleDto {
    ExecPolicyRuleDto {
        id: rule.id().as_str().to_owned(),
        selector: exec_policy_selector_dto(rule.selector()),
        effect: exec_policy_effect_dto(rule.effect()),
        justification: rule.justification().map(str::to_owned),
    }
}

fn exec_policy_rule_from_dto(rule: ExecPolicyRuleDto) -> ExecPolicyRule {
    let mut converted = ExecPolicyRule::new(
        ExecPolicyRuleId::new(rule.id),
        exec_policy_selector_from_dto(rule.selector),
        exec_policy_effect_from_dto(rule.effect),
    );
    if let Some(justification) = rule.justification {
        converted = converted.with_justification(justification);
    }
    converted
}

fn exec_policy_selector_dto(selector: &ExecPolicySelector) -> ExecPolicySelectorDto {
    match selector {
        ExecPolicySelector::Any => ExecPolicySelectorDto::Any,
        ExecPolicySelector::ActionDigest { digest } => ExecPolicySelectorDto::ActionDigest {
            digest: digest.clone(),
        },
        ExecPolicySelector::ActionKind { action_kind } => ExecPolicySelectorDto::ActionKind {
            action_kind: exec_policy_action_kind_dto(*action_kind),
        },
        ExecPolicySelector::Source { source, source_id } => ExecPolicySelectorDto::Source {
            source: source.clone(),
            source_id: source_id.clone(),
        },
        ExecPolicySelector::CommandPrefix { pattern } => ExecPolicySelectorDto::CommandPrefix {
            pattern: pattern.iter().map(exec_policy_token_dto).collect(),
        },
        ExecPolicySelector::Network {
            protocol,
            host,
            port,
        } => ExecPolicySelectorDto::Network {
            protocol: protocol.clone(),
            host: match host {
                HostMatcher::Exact(value) => ExecPolicyHostMatcherDto::Exact(value.clone()),
                HostMatcher::DomainSuffix(value) => {
                    ExecPolicyHostMatcherDto::DomainSuffix(value.clone())
                }
            },
            port: *port,
        },
        ExecPolicySelector::Capability {
            capability_kind,
            scope,
        } => ExecPolicySelectorDto::Capability {
            capability_kind: capability_kind.clone(),
            scope: match scope {
                ScopeMatcher::Exact(value) => ExecPolicyScopeMatcherDto::Exact(value.clone()),
                ScopeMatcher::Prefix(value) => ExecPolicyScopeMatcherDto::Prefix(value.clone()),
            },
        },
        ExecPolicySelector::All { selectors } => ExecPolicySelectorDto::All {
            selectors: selectors.iter().map(exec_policy_selector_dto).collect(),
        },
    }
}

fn exec_policy_selector_from_dto(selector: ExecPolicySelectorDto) -> ExecPolicySelector {
    match selector {
        ExecPolicySelectorDto::Any => ExecPolicySelector::Any,
        ExecPolicySelectorDto::ActionDigest { digest } => {
            ExecPolicySelector::ActionDigest { digest }
        }
        ExecPolicySelectorDto::ActionKind { action_kind } => ExecPolicySelector::ActionKind {
            action_kind: exec_policy_action_kind_from_dto(action_kind),
        },
        ExecPolicySelectorDto::Source { source, source_id } => {
            ExecPolicySelector::source(source, source_id)
        }
        ExecPolicySelectorDto::CommandPrefix { pattern } => {
            ExecPolicySelector::command_prefix(pattern.into_iter().map(exec_policy_token_from_dto))
        }
        ExecPolicySelectorDto::Network {
            protocol,
            host,
            port,
        } => ExecPolicySelector::Network {
            protocol,
            host: match host {
                ExecPolicyHostMatcherDto::Exact(value) => HostMatcher::exact(value),
                ExecPolicyHostMatcherDto::DomainSuffix(value) => HostMatcher::domain_suffix(value),
            },
            port,
        },
        ExecPolicySelectorDto::Capability {
            capability_kind,
            scope,
        } => ExecPolicySelector::Capability {
            capability_kind,
            scope: match scope {
                ExecPolicyScopeMatcherDto::Exact(value) => ScopeMatcher::exact(value),
                ExecPolicyScopeMatcherDto::Prefix(value) => ScopeMatcher::prefix(value),
            },
        },
        ExecPolicySelectorDto::All { selectors } => {
            ExecPolicySelector::all(selectors.into_iter().map(exec_policy_selector_from_dto))
        }
    }
}

fn exec_policy_token_dto(token: &ExecPolicyToken) -> ExecPolicyTokenDto {
    match token {
        ExecPolicyToken::Literal(value) => ExecPolicyTokenDto::Literal(value.clone()),
        ExecPolicyToken::OneOf(values) => {
            ExecPolicyTokenDto::OneOf(values.iter().cloned().collect())
        }
    }
}

fn exec_policy_token_from_dto(token: ExecPolicyTokenDto) -> ExecPolicyToken {
    match token {
        ExecPolicyTokenDto::Literal(value) => ExecPolicyToken::literal(value),
        ExecPolicyTokenDto::OneOf(values) => ExecPolicyToken::one_of(values),
    }
}

fn exec_policy_action_kind_dto(kind: ExecPolicyActionKind) -> ExecPolicyActionKindDto {
    match kind {
        ExecPolicyActionKind::LocalProcess => ExecPolicyActionKindDto::LocalProcess,
        ExecPolicyActionKind::FileSystemMutation => ExecPolicyActionKindDto::FileSystemMutation,
        ExecPolicyActionKind::NetworkRequest => ExecPolicyActionKindDto::NetworkRequest,
        ExecPolicyActionKind::BrowserInteraction => ExecPolicyActionKindDto::BrowserInteraction,
        ExecPolicyActionKind::ExternalServiceMutation => {
            ExecPolicyActionKindDto::ExternalServiceMutation
        }
        ExecPolicyActionKind::CredentialUse => ExecPolicyActionKindDto::CredentialUse,
        ExecPolicyActionKind::SystemOperation => ExecPolicyActionKindDto::SystemOperation,
    }
}

fn exec_policy_action_kind_from_dto(kind: ExecPolicyActionKindDto) -> ExecPolicyActionKind {
    match kind {
        ExecPolicyActionKindDto::LocalProcess => ExecPolicyActionKind::LocalProcess,
        ExecPolicyActionKindDto::FileSystemMutation => ExecPolicyActionKind::FileSystemMutation,
        ExecPolicyActionKindDto::NetworkRequest => ExecPolicyActionKind::NetworkRequest,
        ExecPolicyActionKindDto::BrowserInteraction => ExecPolicyActionKind::BrowserInteraction,
        ExecPolicyActionKindDto::ExternalServiceMutation => {
            ExecPolicyActionKind::ExternalServiceMutation
        }
        ExecPolicyActionKindDto::CredentialUse => ExecPolicyActionKind::CredentialUse,
        ExecPolicyActionKindDto::SystemOperation => ExecPolicyActionKind::SystemOperation,
    }
}

fn exec_policy_effect_dto(effect: &ExecPolicyEffect) -> ExecPolicyEffectDto {
    match effect {
        ExecPolicyEffect::Continue => ExecPolicyEffectDto::Continue,
        ExecPolicyEffect::AllowUnsandboxed => ExecPolicyEffectDto::AllowUnsandboxed,
        ExecPolicyEffect::RequireApproval => ExecPolicyEffectDto::RequireApproval,
        ExecPolicyEffect::RequireSandbox => ExecPolicyEffectDto::RequireSandbox,
        ExecPolicyEffect::Deny(reason) => ExecPolicyEffectDto::Deny(reason.clone()),
    }
}

fn exec_policy_effect_from_dto(effect: ExecPolicyEffectDto) -> ExecPolicyEffect {
    match effect {
        ExecPolicyEffectDto::Continue => ExecPolicyEffect::Continue,
        ExecPolicyEffectDto::AllowUnsandboxed => ExecPolicyEffect::AllowUnsandboxed,
        ExecPolicyEffectDto::RequireApproval => ExecPolicyEffect::RequireApproval,
        ExecPolicyEffectDto::RequireSandbox => ExecPolicyEffect::RequireSandbox,
        ExecPolicyEffectDto::Deny(reason) => ExecPolicyEffect::Deny(reason),
    }
}

fn tool_search_mode_dto(mode: ToolSearchModeConfig) -> ToolSearchModeDto {
    match mode {
        ToolSearchModeConfig::Lexical => ToolSearchModeDto::Lexical,
        ToolSearchModeConfig::HybridEmbedding => ToolSearchModeDto::HybridEmbedding,
    }
}

fn tool_search_status_dto(status: ToolSearchEmbeddingStatus) -> ToolSearchEmbeddingStatusDto {
    match status {
        ToolSearchEmbeddingStatus::Disabled => ToolSearchEmbeddingStatusDto::Disabled,
        ToolSearchEmbeddingStatus::Ready { model } => ToolSearchEmbeddingStatusDto::Ready {
            model: model_ref_dto(model),
        },
        ToolSearchEmbeddingStatus::Unavailable { model, reason } => {
            ToolSearchEmbeddingStatusDto::Unavailable {
                model: model.map(model_ref_dto),
                reason,
            }
        }
    }
}

fn tool_search_config_from_dto(
    mode: ToolSearchModeDto,
    embedding_model: Option<ModelRefDto>,
) -> Result<ToolSearchConfig, RpcError> {
    Ok(ToolSearchConfig {
        mode: match mode {
            ToolSearchModeDto::Lexical => ToolSearchModeConfig::Lexical,
            ToolSearchModeDto::HybridEmbedding => ToolSearchModeConfig::HybridEmbedding,
        },
        embedding_model: embedding_model.map(model_ref_from_dto).transpose()?,
    })
}

fn codebase_models_dto(models: CodebaseModelSelection) -> CodebaseModelsDto {
    CodebaseModelsDto {
        embedding_model: model_ref_dto(models.embedding_model),
        rerank_model: models.rerank_model.map(model_ref_dto),
    }
}

fn codebase_models_from_dto(models: CodebaseModelsDto) -> Result<CodebaseModelSelection, RpcError> {
    Ok(CodebaseModelSelection {
        embedding_model: model_ref_from_dto(models.embedding_model)?,
        rerank_model: models.rerank_model.map(model_ref_from_dto).transpose()?,
    })
}

fn semantic_automatic_context_dto(
    automatic_context: CodebaseAutomaticContext,
) -> CodebaseAutomaticContextDto {
    match automatic_context {
        CodebaseAutomaticContext::Off => CodebaseAutomaticContextDto::Off,
        CodebaseAutomaticContext::FirstInvocation => CodebaseAutomaticContextDto::FirstInvocation,
    }
}

fn semantic_automatic_context_from_dto(
    automatic_context: CodebaseAutomaticContextDto,
) -> CodebaseAutomaticContext {
    match automatic_context {
        CodebaseAutomaticContextDto::Off => CodebaseAutomaticContext::Off,
        CodebaseAutomaticContextDto::FirstInvocation => CodebaseAutomaticContext::FirstInvocation,
    }
}

fn language_server_config_dto(config: LanguageServerConfig) -> LanguageServerConfigDto {
    LanguageServerConfigDto {
        mode: match config.mode {
            LanguageServerModeConfig::Disabled => LanguageServerModeDto::Disabled,
            LanguageServerModeConfig::Automatic => LanguageServerModeDto::Automatic,
            LanguageServerModeConfig::Enabled => LanguageServerModeDto::Enabled,
        },
        executable: config
            .executable
            .map(|path| path.to_string_lossy().into_owned()),
    }
}

fn language_server_config_from_dto(config: LanguageServerConfigDto) -> LanguageServerConfig {
    LanguageServerConfig {
        mode: match config.mode {
            LanguageServerModeDto::Disabled => LanguageServerModeConfig::Disabled,
            LanguageServerModeDto::Automatic => LanguageServerModeConfig::Automatic,
            LanguageServerModeDto::Enabled => LanguageServerModeConfig::Enabled,
        },
        executable: config.executable.map(Into::into),
    }
}

pub(super) fn config_command_result(
    outcome: zeta_config::ConfigCommandResult,
) -> ConfigCommandResult {
    ConfigCommandResult {
        revision: outcome.revision.get(),
        generation: outcome.generation.get(),
        disposition: match outcome.disposition {
            ConfigCommandDisposition::Updated => ConfigCommandDispositionDto::Updated,
            ConfigCommandDisposition::Replayed => ConfigCommandDispositionDto::Replayed,
        },
    }
}

fn model_ref_dto(model_ref: ModelRef) -> ModelRefDto {
    ModelRefDto {
        provider: model_ref.provider.to_string(),
        model: model_ref.model.to_string(),
    }
}

fn model_ref_from_dto(model_ref: ModelRefDto) -> Result<ModelRef, RpcError> {
    Ok(ModelRef::new(
        ProviderId::new(model_ref.provider)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?,
        ModelId::new(model_ref.model)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?,
    ))
}

fn model_ref_update_from_dto(update: Patch<ModelRefDto>) -> Result<Patch<ModelRef>, RpcError> {
    match update {
        Patch::Missing => Ok(Patch::Missing),
        Patch::Null => Ok(Patch::Null),
        Patch::Value(model_ref) => model_ref_from_dto(model_ref).map(Patch::Value),
    }
}

fn approval_review_model_dto(
    selection: ApprovalReviewModelSelection,
) -> ApprovalReviewModelSelectionDto {
    match selection {
        ApprovalReviewModelSelection::Automatic => ApprovalReviewModelSelectionDto::Automatic,
        ApprovalReviewModelSelection::Explicit { model } => {
            ApprovalReviewModelSelectionDto::Explicit {
                model: model_ref_dto(model),
            }
        }
    }
}

fn approval_review_model_from_dto(
    selection: ApprovalReviewModelSelectionDto,
) -> Result<ApprovalReviewModelSelection, RpcError> {
    match selection {
        ApprovalReviewModelSelectionDto::Automatic => Ok(ApprovalReviewModelSelection::Automatic),
        ApprovalReviewModelSelectionDto::Explicit { model } => {
            Ok(ApprovalReviewModelSelection::Explicit {
                model: model_ref_from_dto(model)?,
            })
        }
    }
}

fn approval_review_model_update_from_dto(
    update: Patch<ApprovalReviewModelSelectionDto>,
) -> Result<Patch<ApprovalReviewModelSelection>, RpcError> {
    match update {
        Patch::Missing => Ok(Patch::Missing),
        Patch::Null => Ok(Patch::Null),
        Patch::Value(selection) => approval_review_model_from_dto(selection).map(Patch::Value),
    }
}

fn provider_config_dto(config: ModelProviderConfig) -> ProviderConfigDto {
    ProviderConfigDto {
        provider: config.provider.to_string(),
        base_url: config.base_url,
        max_output_tokens: config.max_output_tokens,
        model_context: config
            .model_context
            .into_iter()
            .map(|(model, context)| {
                (
                    model.to_string(),
                    ModelContextConfigDto {
                        context_window: context.context_window,
                        auto_compact_token_limit: context.auto_compact_token_limit,
                    },
                )
            })
            .collect(),
    }
}

fn provider_config_from_dto(config: ProviderConfigDto) -> Result<ModelProviderConfig, RpcError> {
    let model_context = config
        .model_context
        .into_iter()
        .map(|(model, context)| {
            Ok((
                ModelId::new(model)
                    .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?,
                ModelContextConfig {
                    context_window: context.context_window,
                    auto_compact_token_limit: context.auto_compact_token_limit,
                },
            ))
        })
        .collect::<Result<_, RpcError>>()?;
    Ok(ModelProviderConfig {
        provider: ProviderId::new(config.provider)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?,
        base_url: config.base_url,
        max_output_tokens: config.max_output_tokens,
        model_context,
    })
}

fn mcp_server_config_dto(config: McpServerConfig) -> McpServerConfigDto {
    McpServerConfigDto {
        id: config.id.to_string(),
        display_name: config.display_name,
        transport: mcp_transport_dto(config.transport),
        credential: mcp_credential_dto(config.credential),
        enablement: mcp_enablement_dto(config.enablement),
    }
}

fn mcp_server_config_from_dto(config: McpServerConfigDto) -> Result<McpServerConfig, RpcError> {
    Ok(McpServerConfig {
        id: McpServerId::new(config.id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?,
        display_name: config.display_name,
        transport: mcp_transport_from_dto(config.transport),
        credential: mcp_credential_from_dto(config.credential),
        enablement: mcp_enablement_from_dto(config.enablement),
    })
}

fn mcp_transport_dto(transport: McpTransportConfig) -> McpTransportDto {
    match transport {
        McpTransportConfig::Stdio { command, args } => McpTransportDto::Stdio { command, args },
        McpTransportConfig::StreamableHttp { url } => McpTransportDto::StreamableHttp { url },
    }
}

fn mcp_transport_from_dto(transport: McpTransportDto) -> McpTransportConfig {
    match transport {
        McpTransportDto::Stdio { command, args } => McpTransportConfig::Stdio { command, args },
        McpTransportDto::StreamableHttp { url } => McpTransportConfig::StreamableHttp { url },
    }
}

fn mcp_credential_dto(credential: McpCredentialBinding) -> McpCredentialBindingDto {
    match credential {
        McpCredentialBinding::Unauthenticated => McpCredentialBindingDto::Unauthenticated,
        McpCredentialBinding::Reference { credential_ref } => {
            McpCredentialBindingDto::Reference { credential_ref }
        }
    }
}

fn mcp_credential_from_dto(credential: McpCredentialBindingDto) -> McpCredentialBinding {
    match credential {
        McpCredentialBindingDto::Unauthenticated => McpCredentialBinding::Unauthenticated,
        McpCredentialBindingDto::Reference { credential_ref } => {
            McpCredentialBinding::Reference { credential_ref }
        }
    }
}

fn mcp_enablement_dto(enablement: McpServerEnablement) -> McpServerEnablementDto {
    match enablement {
        McpServerEnablement::Disabled => McpServerEnablementDto::Disabled,
        McpServerEnablement::Enabled => McpServerEnablementDto::Enabled,
    }
}

fn mcp_enablement_from_dto(enablement: McpServerEnablementDto) -> McpServerEnablement {
    match enablement {
        McpServerEnablementDto::Disabled => McpServerEnablement::Disabled,
        McpServerEnablementDto::Enabled => McpServerEnablement::Enabled,
    }
}

fn skill_source_config_dto(config: SkillSourceConfig) -> SkillSourceConfigDto {
    SkillSourceConfigDto {
        id: config.id.to_string(),
        root_reference: config.root_reference,
        enablement: skill_enablement_dto(config.enablement),
    }
}

fn skill_source_config_from_dto(
    config: SkillSourceConfigDto,
) -> Result<SkillSourceConfig, RpcError> {
    Ok(SkillSourceConfig {
        id: SkillSourceId::new(config.id)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?,
        root_reference: config.root_reference,
        enablement: skill_enablement_from_dto(config.enablement),
    })
}

fn skill_enablement_dto(enablement: SkillSourceEnablement) -> SkillSourceEnablementDto {
    match enablement {
        SkillSourceEnablement::Disabled => SkillSourceEnablementDto::Disabled,
        SkillSourceEnablement::Enabled => SkillSourceEnablementDto::Enabled,
    }
}

fn skill_enablement_from_dto(enablement: SkillSourceEnablementDto) -> SkillSourceEnablement {
    match enablement {
        SkillSourceEnablementDto::Disabled => SkillSourceEnablement::Disabled,
        SkillSourceEnablementDto::Enabled => SkillSourceEnablement::Enabled,
    }
}
