use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::config_operations::config_command_result;
use super::config_operations::config_operation_error;
use super::decode;
use super::result;
use super::workspace_runtime::WorkspaceRuntimeError;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryAddParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryMutationDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryMutationResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryPermissionsSetParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryRemoveParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceFolderDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceFoldersSetParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceFoldersSetResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceSwitchParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceSwitchResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceSwitchTrust;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustEntryDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustForgetParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustListResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustReadParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustReadResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustSetParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustSettingDto;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustStateDto;
use zeta_config::ConfigCommandRequest;
use zeta_config::ConfigRevision;
use zeta_config::UserConfigCommand;
use zeta_config::WorkspaceTrustSetting;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace::WorkspaceTrustSource;
use zeta_workspace_access::AdditionalDirectoryPermission;
use zeta_workspace_access::AdditionalDirectoryPermissions;
use zeta_workspace_access::WorkspaceAccessMutation;

impl AppServer {
    pub(super) fn workspace_additional_directory_list(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let params: WorkspaceAdditionalDirectoryListParams = decode(params)?;
        let snapshot = self
            .list_session_additional_directories(&params.session_id)
            .map_err(workspace_runtime_error)?;
        result(&WorkspaceAdditionalDirectoryListResult {
            revision: snapshot.revision,
            directories: additional_directory_dtos(snapshot.directories),
        })
    }

    pub(super) fn workspace_additional_directory_add(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let params: WorkspaceAdditionalDirectoryAddParams = decode(params)?;
        if params.root.as_os_str().is_empty() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let permissions = additional_directory_permissions(params.permissions)?;
        let (mutation, snapshot) = self
            .add_session_additional_directory(&params.session_id, params.root, permissions)
            .map_err(workspace_runtime_error)?;
        result(&WorkspaceAdditionalDirectoryMutationResult {
            mutation: additional_directory_mutation(mutation),
            revision: snapshot.revision,
            directories: additional_directory_dtos(snapshot.directories),
        })
    }

    pub(super) fn workspace_additional_directory_remove(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let params: WorkspaceAdditionalDirectoryRemoveParams = decode(params)?;
        if params.root.as_os_str().is_empty() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let (mutation, snapshot) = self
            .remove_session_additional_directory(&params.session_id, &params.root)
            .map_err(workspace_runtime_error)?;
        result(&WorkspaceAdditionalDirectoryMutationResult {
            mutation: additional_directory_mutation(mutation),
            revision: snapshot.revision,
            directories: additional_directory_dtos(snapshot.directories),
        })
    }

    pub(super) fn workspace_additional_directory_permissions_set(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let params: WorkspaceAdditionalDirectoryPermissionsSetParams = decode(params)?;
        if params.root.as_os_str().is_empty() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let permissions = additional_directory_permissions(params.permissions)?;
        let (mutation, snapshot) = self
            .set_session_additional_directory_permissions(
                &params.session_id,
                &params.root,
                params.expected_revision,
                permissions,
            )
            .map_err(workspace_runtime_error)?;
        result(&WorkspaceAdditionalDirectoryMutationResult {
            mutation: additional_directory_mutation(mutation),
            revision: snapshot.revision,
            directories: additional_directory_dtos(snapshot.directories),
        })
    }

    pub(super) fn workspace_trust_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let params: WorkspaceTrustReadParams = decode(params)?;
        if !params.root.is_absolute() || params.root.as_os_str().is_empty() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let workspace = WorkspaceRoot::open(params.root)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let setting = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
            .read_snapshot()
            .map_err(|_| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
            .values
            .workspace_trust
            .explicit_setting_for(&workspace.trust_id())
            .map(|setting| match setting {
                WorkspaceTrustSetting::Restricted => WorkspaceTrustSettingDto::Restricted,
                WorkspaceTrustSetting::Trusted => WorkspaceTrustSettingDto::Trusted,
            });
        let state = if setting == Some(WorkspaceTrustSettingDto::Trusted) {
            WorkspaceTrustStateDto::Trusted
        } else {
            WorkspaceTrustStateDto::Restricted
        };
        result(&WorkspaceTrustReadResult { setting, state })
    }

    pub(super) fn workspace_trust_list(
        &self,
        connection: &ConnectionState,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let snapshot = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
            .read_snapshot()
            .map_err(|_| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let entries = snapshot
            .values
            .workspace_trust
            .roots
            .iter()
            .filter(|(_, setting)| **setting == WorkspaceTrustSetting::Trusted)
            .map(|(workspace, _)| WorkspaceTrustEntryDto {
                workspace: workspace.clone(),
                root: snapshot
                    .values
                    .workspace_trust
                    .explicit_root_path_for(workspace)
                    .map(|path| path.to_path_buf()),
            })
            .collect();
        result(&WorkspaceTrustListResult {
            revision: snapshot.revision.get(),
            entries,
        })
    }

    pub(super) fn workspace_trust_set(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let params: WorkspaceTrustSetParams = decode(params)?;
        if !params.root.is_absolute() || params.root.as_os_str().is_empty() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let workspace = WorkspaceRoot::open(params.root)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        let setting = workspace_trust_setting(params.setting);
        let workspace_id = workspace.trust_id();
        let command = match setting {
            WorkspaceTrustSetting::Restricted => UserConfigCommand::ForgetWorkspaceTrust {
                workspace: workspace_id,
            },
            WorkspaceTrustSetting::Trusted => UserConfigCommand::SetWorkspaceTrust {
                workspace: workspace_id,
                setting,
                display_root: Some(workspace.canonical_path().to_path_buf()),
            },
        };
        let outcome = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command,
            })
            .map_err(config_operation_error)?;
        let command_result = config_command_result(outcome);
        self.reconcile_active_workspace_trust()?;
        result(&command_result)
    }

    pub(super) fn workspace_trust_forget(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        require_workspace_trust_host(connection)?;
        let params: WorkspaceTrustForgetParams = decode(params)?;
        let outcome = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
            .apply(ConfigCommandRequest {
                command_id: params.command_id,
                expected_revision: ConfigRevision::new(params.expected_revision),
                command: UserConfigCommand::ForgetWorkspaceTrust {
                    workspace: params.workspace,
                },
            })
            .map_err(config_operation_error)?;
        let command_result = config_command_result(outcome);
        self.reconcile_active_workspace_trust()?;
        result(&command_result)
    }

    pub(super) fn workspace_switch(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceSwitchParams = decode(params)?;
        if !params.root.is_absolute() || params.root.as_os_str().is_empty() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let decision =
            self.resolve_workspace_switch_trust(connection, &params.root, params.trust)?;
        let root = match decision {
            Some(decision) => self
                .switch_local_workspace_root_with_decision(params.root, decision)
                .map_err(workspace_runtime_error)?,
            None => self
                .switch_local_workspace_root(params.root)
                .map_err(workspace_runtime_error)?,
        };
        let trust = if self.active_workspace_is_trusted() {
            WorkspaceTrustStateDto::Trusted
        } else {
            WorkspaceTrustStateDto::Restricted
        };
        result(&WorkspaceSwitchResult { root, trust })
    }

    pub(super) fn workspace_folders_set(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceFoldersSetParams = decode(params)?;
        if params.folders.is_empty() || params.folders.len() > 256 {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut folders = Vec::with_capacity(params.folders.len());
        for folder in params.folders {
            if folder.id.trim().is_empty()
                || folder.id.len() > 256
                || !ids.insert(folder.id.clone())
                || !folder.root.is_absolute()
                || folder.root.as_os_str().is_empty()
            {
                return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
            }
            let decision =
                self.resolve_workspace_switch_trust(connection, &folder.root, folder.trust)?;
            let authorization = self
                .authorize_local_workspace_root(folder.root, decision)
                .map_err(workspace_runtime_error)?;
            folders.push((folder.id, authorization));
        }
        let folders = self
            .activate_local_workspace_folders(folders)
            .map_err(workspace_runtime_error)?
            .into_iter()
            .map(|(id, root, decision)| WorkspaceFolderDto {
                id,
                root,
                trust: workspace_trust_state(decision),
            })
            .collect();
        result(&WorkspaceFoldersSetResult { folders })
    }

    fn resolve_workspace_switch_trust(
        &self,
        connection: &ConnectionState,
        root: &std::path::Path,
        trust: WorkspaceSwitchTrust,
    ) -> Result<Option<WorkspaceTrustDecision>, RpcError> {
        Ok(match trust {
            WorkspaceSwitchTrust::UserConfig => None,
            WorkspaceSwitchTrust::HostSession => {
                require_workspace_trust_host(connection)?;
                Some(WorkspaceTrustDecision::Trusted(
                    WorkspaceTrustSource::HostConfiguration,
                ))
            }
            WorkspaceSwitchTrust::UserDecision {
                command_id,
                expected_revision,
                setting,
            } => {
                require_workspace_trust_host(connection)?;
                let workspace = WorkspaceRoot::open(root.to_path_buf())
                    .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
                let setting = match setting {
                    WorkspaceTrustSettingDto::Restricted => WorkspaceTrustSetting::Restricted,
                    WorkspaceTrustSettingDto::Trusted => WorkspaceTrustSetting::Trusted,
                };
                let workspace_id = workspace.trust_id();
                let command = match setting {
                    WorkspaceTrustSetting::Restricted => UserConfigCommand::ForgetWorkspaceTrust {
                        workspace: workspace_id,
                    },
                    WorkspaceTrustSetting::Trusted => UserConfigCommand::SetWorkspaceTrust {
                        workspace: workspace_id,
                        setting,
                        display_root: Some(workspace.canonical_path().to_path_buf()),
                    },
                };
                self.config
                    .as_ref()
                    .ok_or_else(|| {
                        RpcError::new(-32070, AppServerErrorName::WorkspaceSwitchUnavailable)
                    })?
                    .apply(ConfigCommandRequest {
                        command_id,
                        expected_revision: ConfigRevision::new(expected_revision),
                        command,
                    })
                    .map_err(|_| {
                        RpcError::new(-32072, AppServerErrorName::WorkspaceSwitchFailed)
                    })?;
                Some(setting.into_decision())
            }
        })
    }
}

fn additional_directory_dtos(
    directories: Vec<super::workspace_runtime::SessionAdditionalDirectorySnapshot>,
) -> Vec<WorkspaceAdditionalDirectoryDto> {
    directories
        .into_iter()
        .map(|directory| {
            let contributions =
                additional_directory_contributions(&directory.root, &directory.permissions);
            WorkspaceAdditionalDirectoryDto {
                root: directory.root,
                trust: workspace_trust_state(directory.decision),
                permissions: directory
                    .permissions
                    .entries()
                    .map(additional_directory_permission_dto)
                    .collect(),
                contributions,
            }
        })
        .collect()
}

fn additional_directory_contributions(
    root: &std::path::Path,
    permissions: &AdditionalDirectoryPermissions,
) -> zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryContributionsDto {
    use zeta_app_server_protocol::protocol::workspace::WorkspaceAdditionalDirectoryContributionsDto;
    use zeta_workspace_access::AdditionalDirectoryPermission as Permission;

    let mut result = WorkspaceAdditionalDirectoryContributionsDto::default();
    if permissions.allows(Permission::DiscoverSkills) {
        let skill_root = root.join(".zeta/skills");
        if let Ok(entries) = std::fs::read_dir(skill_root) {
            result
                .skills
                .extend(entries.filter_map(Result::ok).filter_map(|entry| {
                    entry
                        .path()
                        .join("SKILL.md")
                        .is_file()
                        .then(|| entry.file_name().to_string_lossy().into_owned())
                }));
        }
    }
    let needs_config = permissions.allows(Permission::DiscoverSkills)
        || permissions.allows(Permission::DiscoverMcp)
        || permissions.allows(Permission::DiscoverHooks)
        || permissions.allows(Permission::DiscoverPlugins);
    if !needs_config {
        return result;
    }
    let workspace = match zeta_workspace::WorkspaceRoot::open(root) {
        Ok(workspace) => workspace,
        Err(error) => {
            result.diagnostics.push(error.to_string());
            return result;
        }
    };
    let document = match super::workspace_runtime::read_additional_workspace_config(&workspace) {
        Ok(document) => document,
        Err(error) => {
            result.diagnostics.push(error.to_string());
            return result;
        }
    };
    if permissions.allows(Permission::DiscoverMcp) {
        result
            .mcp_servers
            .extend(document.mcp.servers.keys().map(ToString::to_string));
    }
    if permissions.allows(Permission::DiscoverHooks) {
        result
            .hooks
            .extend(document.hooks.hooks.keys().map(ToString::to_string));
    }
    if permissions.allows(Permission::DiscoverPlugins) {
        result.plugins.extend(
            document
                .plugin_requests
                .requests
                .keys()
                .map(ToString::to_string),
        );
    }
    result.skills.sort();
    result.skills.dedup();
    result.mcp_servers.sort();
    result.hooks.sort();
    result.plugins.sort();
    result
}

fn additional_directory_mutation(
    mutation: WorkspaceAccessMutation,
) -> WorkspaceAdditionalDirectoryMutationDto {
    match mutation {
        WorkspaceAccessMutation::AddedDirectory | WorkspaceAccessMutation::AddedSource => {
            WorkspaceAdditionalDirectoryMutationDto::Added
        }
        WorkspaceAccessMutation::AlreadyPresent => {
            WorkspaceAdditionalDirectoryMutationDto::AlreadyPresent
        }
        WorkspaceAccessMutation::RemovedDirectory | WorkspaceAccessMutation::RemovedSource => {
            WorkspaceAdditionalDirectoryMutationDto::Removed
        }
        WorkspaceAccessMutation::UpdatedPermissions => {
            WorkspaceAdditionalDirectoryMutationDto::Updated
        }
        WorkspaceAccessMutation::NotPresent => WorkspaceAdditionalDirectoryMutationDto::NotPresent,
    }
}

fn additional_directory_permissions(
    values: Vec<WorkspaceAdditionalDirectoryPermissionDto>,
) -> Result<AdditionalDirectoryPermissions, RpcError> {
    let permissions = values
        .into_iter()
        .map(additional_directory_permission)
        .collect::<Vec<_>>();
    let unique = permissions
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != permissions.len() {
        return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
    }
    AdditionalDirectoryPermissions::new(permissions)
        .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))
}

fn additional_directory_permission(
    value: WorkspaceAdditionalDirectoryPermissionDto,
) -> AdditionalDirectoryPermission {
    match value {
        WorkspaceAdditionalDirectoryPermissionDto::ReadFiles => {
            AdditionalDirectoryPermission::ReadFiles
        }
        WorkspaceAdditionalDirectoryPermissionDto::WriteFiles => {
            AdditionalDirectoryPermission::WriteFiles
        }
        WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands => {
            AdditionalDirectoryPermission::ExecuteCommands
        }
        WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges => {
            AdditionalDirectoryPermission::WatchFileChanges
        }
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceFiles => {
            AdditionalDirectoryPermission::UseWorkspaceFiles
        }
        WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch => {
            AdditionalDirectoryPermission::UseWorkspaceSearch
        }
        WorkspaceAdditionalDirectoryPermissionDto::LoadInstructionsAndAgents => {
            AdditionalDirectoryPermission::LoadInstructionsAndAgents
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverSkills => {
            AdditionalDirectoryPermission::DiscoverSkills
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverMcp => {
            AdditionalDirectoryPermission::DiscoverMcp
        }
        WorkspaceAdditionalDirectoryPermissionDto::UseLanguageServices => {
            AdditionalDirectoryPermission::UseLanguageServices
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverHooks => {
            AdditionalDirectoryPermission::DiscoverHooks
        }
        WorkspaceAdditionalDirectoryPermissionDto::DiscoverPlugins => {
            AdditionalDirectoryPermission::DiscoverPlugins
        }
    }
}

fn additional_directory_permission_dto(
    value: AdditionalDirectoryPermission,
) -> WorkspaceAdditionalDirectoryPermissionDto {
    match value {
        AdditionalDirectoryPermission::ReadFiles => {
            WorkspaceAdditionalDirectoryPermissionDto::ReadFiles
        }
        AdditionalDirectoryPermission::WriteFiles => {
            WorkspaceAdditionalDirectoryPermissionDto::WriteFiles
        }
        AdditionalDirectoryPermission::ExecuteCommands => {
            WorkspaceAdditionalDirectoryPermissionDto::ExecuteCommands
        }
        AdditionalDirectoryPermission::WatchFileChanges => {
            WorkspaceAdditionalDirectoryPermissionDto::WatchFileChanges
        }
        AdditionalDirectoryPermission::UseWorkspaceFiles => {
            WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceFiles
        }
        AdditionalDirectoryPermission::UseWorkspaceSearch => {
            WorkspaceAdditionalDirectoryPermissionDto::UseWorkspaceSearch
        }
        AdditionalDirectoryPermission::LoadInstructionsAndAgents => {
            WorkspaceAdditionalDirectoryPermissionDto::LoadInstructionsAndAgents
        }
        AdditionalDirectoryPermission::DiscoverSkills => {
            WorkspaceAdditionalDirectoryPermissionDto::DiscoverSkills
        }
        AdditionalDirectoryPermission::DiscoverMcp => {
            WorkspaceAdditionalDirectoryPermissionDto::DiscoverMcp
        }
        AdditionalDirectoryPermission::UseLanguageServices => {
            WorkspaceAdditionalDirectoryPermissionDto::UseLanguageServices
        }
        AdditionalDirectoryPermission::DiscoverHooks => {
            WorkspaceAdditionalDirectoryPermissionDto::DiscoverHooks
        }
        AdditionalDirectoryPermission::DiscoverPlugins => {
            WorkspaceAdditionalDirectoryPermissionDto::DiscoverPlugins
        }
    }
}

impl AppServer {
    pub(crate) fn reconcile_active_workspace_trust(&self) -> Result<(), RpcError> {
        if self.local_workspace_host.is_none() {
            return Ok(());
        }
        let active = self
            .workspace_runtime
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .authorization
            .as_ref()
            .map(|authorization| (authorization.root().clone(), authorization.decision()));
        let Some(runtime) = self.workspace_runtime_control() else {
            return Ok(());
        };
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        let snapshot = config
            .read_snapshot()
            .map_err(|_| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?;
        if let Some((root, WorkspaceTrustDecision::Restricted)) = active
            && matches!(
                snapshot
                    .values
                    .workspace_trust
                    .decision_for(&root.trust_id()),
                WorkspaceTrustDecision::Trusted(_)
            )
        {
            self.switch_local_workspace_root_with_decision(
                root.canonical_path().to_path_buf(),
                WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision),
            )
            .map(|_| ())
            .map_err(workspace_runtime_error)?;
            return Ok(());
        }
        runtime
            .reconcile_user_trust(&snapshot.values)
            .map_err(workspace_runtime_error)
    }
}

fn workspace_trust_setting(setting: WorkspaceTrustSettingDto) -> WorkspaceTrustSetting {
    match setting {
        WorkspaceTrustSettingDto::Restricted => WorkspaceTrustSetting::Restricted,
        WorkspaceTrustSettingDto::Trusted => WorkspaceTrustSetting::Trusted,
    }
}

fn workspace_trust_state(decision: WorkspaceTrustDecision) -> WorkspaceTrustStateDto {
    if decision == WorkspaceTrustDecision::Restricted {
        WorkspaceTrustStateDto::Restricted
    } else {
        WorkspaceTrustStateDto::Trusted
    }
}

fn require_workspace_trust_host(connection: &ConnectionState) -> Result<(), RpcError> {
    if connection.supports_workspace_trust_host() {
        Ok(())
    } else {
        Err(RpcError::new(
            -32073,
            AppServerErrorName::WorkspaceTrustRequired,
        ))
    }
}

fn workspace_runtime_error(error: WorkspaceRuntimeError) -> RpcError {
    match error {
        WorkspaceRuntimeError::Unavailable => {
            RpcError::new(-32070, AppServerErrorName::WorkspaceSwitchUnavailable)
        }
        WorkspaceRuntimeError::Busy => {
            RpcError::new(-32071, AppServerErrorName::WorkspaceSwitchBusy)
        }
        WorkspaceRuntimeError::AccessRevisionConflict => {
            RpcError::new(-32074, AppServerErrorName::WorkspaceAccessRevisionConflict)
        }
        WorkspaceRuntimeError::TrustRequired => {
            RpcError::new(-32073, AppServerErrorName::WorkspaceTrustRequired)
        }
        WorkspaceRuntimeError::Failed(_) => {
            RpcError::new(-32072, AppServerErrorName::WorkspaceSwitchFailed)
        }
    }
}
