use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use super::result;
use super::workspace_runtime::WorkspaceRuntimeError;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::workspace::WorkspaceSwitchParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceSwitchResult;
use zeta_app_server_protocol::protocol::workspace::WorkspaceSwitchTrust;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustReadParams;
use zeta_app_server_protocol::protocol::workspace::WorkspaceTrustReadResult;
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
        result(&WorkspaceTrustReadResult { setting })
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
                self.config
                    .as_ref()
                    .ok_or_else(|| {
                        RpcError::new(-32070, AppServerErrorName::WorkspaceSwitchUnavailable)
                    })?
                    .apply(ConfigCommandRequest {
                        command_id,
                        expected_revision: ConfigRevision::new(expected_revision),
                        command: UserConfigCommand::SetWorkspaceTrust {
                            workspace: workspace.trust_id(),
                            setting,
                        },
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
