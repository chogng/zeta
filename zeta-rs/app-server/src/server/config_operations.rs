use super::extension_config_operations::{hook_config_dto, plugin_request_dto};
use super::{AppServer, RpcError, decode, result};
use serde_json::Value;
use zeta_app_server_protocol::protocol::config::{
    ApprovalReviewModelSelectionDto, ConfigCommandDispositionDto, ConfigCommandResult,
    ConfigReadResult, ConfigUpdateParams, LanguageServerConfigDto, LanguageServerConfigureParams,
    LanguageServerModeDto, LanguageServerRemoveParams, McpCredentialBindingDto, McpServerConfigDto,
    McpServerEnablementDto, McpServerRemoveParams, McpServerSetEnablementParams,
    McpServerUpsertParams, McpTransportDto, ModelContextConfigDto, ModelRefDto, ProviderConfigDto,
    ProviderConfigureParams, ProviderRemoveParams, SkillSourceAddParams, SkillSourceConfigDto,
    SkillSourceEnablementDto, SkillSourceRemoveParams, SkillSourceSetEnablementParams,
};
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_config::{
    ApprovalReviewModelSelection, ConfigCommandDisposition, ConfigCommandError,
    ConfigCommandRequest, ConfigRevision, LanguageServerConfig, LanguageServerId,
    LanguageServerModeConfig, McpCredentialBinding, McpServerConfig, McpServerEnablement,
    McpServerId, McpTransportConfig, PreferencesUpdate, ResolvedConfigSnapshot, SkillSourceConfig,
    SkillSourceEnablement, SkillSourceId, UserConfigCommand,
};
use zeta_model_provider::{ModelId, ModelRef, ProviderId};
use zeta_model_provider_config::ModelContextConfig;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::Patch;

impl AppServer {
    pub(super) fn config_read(&self) -> Result<Value, RpcError> {
        let snapshot = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
            .read_snapshot()
            .map_err(config_error)?;
        result(&config_read_result(snapshot))
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
                }),
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

fn config_read_result(snapshot: ResolvedConfigSnapshot) -> ConfigReadResult {
    ConfigReadResult {
        revision: snapshot.revision.get(),
        generation: snapshot.generation.get(),
        preferred_model: snapshot.values.preferred_model.map(model_ref_dto),
        approval_review_model: approval_review_model_dto(snapshot.values.approval_review_model),
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
