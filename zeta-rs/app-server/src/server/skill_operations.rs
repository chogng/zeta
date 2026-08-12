use super::config_operations::{config_command_result, config_operation_error};
use super::{AppServer, RpcError, decode, result};
use serde_json::Value;
use zeta_app_server_protocol::protocol::config::ConfigCommandResult;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::skills::{
    SkillCatalogReloadDto, SkillCompatibilityDto, SkillDiagnosticCodeDto, SkillDiagnosticDto,
    SkillDto, SkillEnablementDto, SkillListParams, SkillListResult, SkillSetEnablementParams,
    SkillSourceKindDto,
};
use zeta_config::{ConfigCommandRequest, ConfigRevision, SkillEnablement, UserConfigCommand};
use zeta_skills_extension::SkillCatalogReload;
use zeta_skills_extension::SkillCompatibility;
use zeta_skills_extension::SkillDiagnosticCode;
use zeta_skills_extension::SkillRuntimeDiagnostic;
use zeta_skills_extension::SkillRuntimeSnapshot;
use zeta_skills_extension::SkillSourceKind;

impl AppServer {
    pub(super) fn skill_list(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SkillListParams = decode(params)?;
        let runtime = self.skills.as_ref().ok_or_else(skills_unavailable)?;
        let reload = match params.reload {
            SkillCatalogReloadDto::Cached => SkillCatalogReload::Cached,
            SkillCatalogReloadDto::Refresh => SkillCatalogReload::Refresh,
        };
        let snapshot = runtime.list(reload).map_err(skills_failed)?;
        result(&skill_list_result(snapshot.as_ref()))
    }

    pub(super) fn skill_set_enablement(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SkillSetEnablementParams = decode(params)?;
        let runtime = self.skills.as_ref().ok_or_else(skills_unavailable)?;
        let snapshot = runtime
            .list(SkillCatalogReload::Cached)
            .map_err(skills_failed)?;
        if !snapshot
            .entries
            .iter()
            .any(|entry| entry.catalog_entry.id() == &params.skill_id)
        {
            return Err(RpcError::new(-32052, AppServerErrorName::SkillNotFound));
        }
        let store = self
            .config
            .clone()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let outcome = store
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::SetSkillEnablement {
                    skill_id: params.skill_id,
                    enablement: enablement_from_dto(params.enablement),
                },
            })
            .map_err(config_operation_error)?;
        runtime
            .list(SkillCatalogReload::Cached)
            .map_err(skills_failed)?;
        result::<ConfigCommandResult>(&config_command_result(outcome))
    }
}

fn skill_list_result(snapshot: &SkillRuntimeSnapshot) -> SkillListResult {
    SkillListResult {
        generation: snapshot.generation,
        skills: snapshot
            .entries
            .iter()
            .map(|entry| {
                let catalog = &entry.catalog_entry;
                SkillDto {
                    id: catalog.id().clone(),
                    description: catalog.metadata().description().to_owned(),
                    source_kind: match catalog.source().kind() {
                        SkillSourceKind::BuiltIn => SkillSourceKindDto::BuiltIn,
                        SkillSourceKind::User => SkillSourceKindDto::User,
                        SkillSourceKind::Workspace => SkillSourceKindDto::Workspace,
                    },
                    content_digest: catalog.content_digest().clone(),
                    enablement: enablement_dto(entry.enablement),
                    compatibility: match catalog.compatibility() {
                        SkillCompatibility::Compatible => SkillCompatibilityDto::Compatible,
                        SkillCompatibility::Unknown { note } => {
                            SkillCompatibilityDto::Unknown { note: note.clone() }
                        }
                    },
                }
            })
            .collect(),
        diagnostics: snapshot.diagnostics.iter().map(diagnostic_dto).collect(),
    }
}

fn diagnostic_dto(diagnostic: &SkillRuntimeDiagnostic) -> SkillDiagnosticDto {
    SkillDiagnosticDto {
        source: diagnostic.source.clone(),
        subject: diagnostic.subject.clone(),
        code: match diagnostic.code {
            SkillDiagnosticCode::SourceUnavailable => SkillDiagnosticCodeDto::SourceUnavailable,
            SkillDiagnosticCode::SourceLimitExceeded => SkillDiagnosticCodeDto::SourceLimitExceeded,
            SkillDiagnosticCode::SkillNotFound => SkillDiagnosticCodeDto::SkillNotFound,
            SkillDiagnosticCode::InvalidFrontmatter => SkillDiagnosticCodeDto::InvalidFrontmatter,
            SkillDiagnosticCode::InvalidSkillName => SkillDiagnosticCodeDto::InvalidSkillName,
            SkillDiagnosticCode::DescriptionInvalid => SkillDiagnosticCodeDto::DescriptionInvalid,
            SkillDiagnosticCode::PathEscapesRoot => SkillDiagnosticCodeDto::PathEscapesRoot,
            SkillDiagnosticCode::UnsupportedFileType => SkillDiagnosticCodeDto::UnsupportedFileType,
            SkillDiagnosticCode::ContentTooLarge => SkillDiagnosticCodeDto::ContentTooLarge,
        },
        message: diagnostic.message.clone(),
    }
}

fn enablement_dto(enablement: SkillEnablement) -> SkillEnablementDto {
    match enablement {
        SkillEnablement::Disabled => SkillEnablementDto::Disabled,
        SkillEnablement::Enabled => SkillEnablementDto::Enabled,
    }
}

fn enablement_from_dto(enablement: SkillEnablementDto) -> SkillEnablement {
    match enablement {
        SkillEnablementDto::Disabled => SkillEnablement::Disabled,
        SkillEnablementDto::Enabled => SkillEnablement::Enabled,
    }
}

fn skills_unavailable() -> RpcError {
    RpcError::new(-32050, AppServerErrorName::SkillsUnavailable)
}

fn skills_failed(_: String) -> RpcError {
    RpcError::new(-32051, AppServerErrorName::SkillOperationFailed)
}
