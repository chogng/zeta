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

impl AppServer {
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
        let decision = match params.trust {
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
                let workspace = WorkspaceRoot::open(params.root.clone())
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
        };
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
        WorkspaceRuntimeError::TrustRequired => {
            RpcError::new(-32073, AppServerErrorName::WorkspaceTrustRequired)
        }
        WorkspaceRuntimeError::Failed(_) => {
            RpcError::new(-32072, AppServerErrorName::WorkspaceSwitchFailed)
        }
    }
}
