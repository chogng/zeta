use super::workspace_runtime::WorkspaceRuntimeError;
use super::{AppServer, RpcError, decode, result};
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::workspace::{WorkspaceSwitchParams, WorkspaceSwitchResult};

impl AppServer {
    pub(super) fn workspace_switch(&self, params: &Value) -> Result<Value, RpcError> {
        let params: WorkspaceSwitchParams = decode(params)?;
        if !params.root.is_absolute() || params.root.as_os_str().is_empty() {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        let root = self
            .switch_local_workspace_root(params.root)
            .map_err(workspace_runtime_error)?;
        result(&WorkspaceSwitchResult { root })
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
        WorkspaceRuntimeError::Failed(_) => {
            RpcError::new(-32072, AppServerErrorName::WorkspaceSwitchFailed)
        }
    }
}
