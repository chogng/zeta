use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::terminal::TerminalAttachParams;
use zeta_app_server_protocol::protocol::terminal::TerminalCloseParams;
use zeta_app_server_protocol::protocol::terminal::TerminalCreateInSessionDirectoryParams;
use zeta_app_server_protocol::protocol::terminal::TerminalCreateParams;
use zeta_app_server_protocol::protocol::terminal::TerminalProfileListResult;
use zeta_app_server_protocol::protocol::terminal::TerminalReadParams;
use zeta_app_server_protocol::protocol::terminal::TerminalResizeParams;
use zeta_app_server_protocol::protocol::terminal::TerminalWriteParams;

impl AppServer {
    pub(super) fn terminal_profile_list(&self, params: &Value) -> Result<Value, RpcError> {
        let _: EmptyParams = decode(params)?;
        result(&TerminalProfileListResult {
            profiles: self.terminal_service()?.profiles(),
        })
    }

    pub(super) fn terminal_create(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalCreateParams = decode(params)?;
        let created = self
            .terminal_service_for(params.workspace_folder_id.as_deref())?
            .create(connection.connection_id, params)
            .map_err(terminal_error)?;
        result(&created)
    }

    pub(super) fn terminal_create_in_session_directory(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalCreateInSessionDirectoryParams = decode(params)?;
        let workspace = self.session_additional_directory_workspace(
            &params.session_id,
            &params.root,
            zeta_workspace::WorkspaceCapability::ExecuteProcess,
        )?;
        let created = self
            .terminal_service()?
            .create_in_workspace(
                connection.connection_id,
                TerminalCreateParams {
                    workspace_folder_id: None,
                    rows: params.rows,
                    cols: params.cols,
                    profile: params.profile,
                    lifecycle: params.lifecycle,
                },
                workspace,
            )
            .map_err(terminal_error)?;
        result(&created)
    }

    pub(super) fn terminal_write(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalWriteParams = decode(params)?;
        self.terminal_service_for(params.workspace_folder_id.as_deref())?
            .write(connection.connection_id, params)
            .map_err(terminal_error)?;
        result(&())
    }

    pub(super) fn terminal_attach(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalAttachParams = decode(params)?;
        let attached = self
            .terminal_service_for(params.workspace_folder_id.as_deref())?
            .attach(connection.connection_id, params)
            .map_err(terminal_error)?;
        result(&attached)
    }

    pub(super) fn terminal_resize(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalResizeParams = decode(params)?;
        self.terminal_service_for(params.workspace_folder_id.as_deref())?
            .resize(connection.connection_id, params)
            .map_err(terminal_error)?;
        result(&())
    }

    pub(super) fn terminal_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalReadParams = decode(params)?;
        let output = self
            .terminal_service_for(params.workspace_folder_id.as_deref())?
            .read(connection.connection_id, params)
            .map_err(terminal_error)?;
        result(&output)
    }

    pub(super) fn terminal_close(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalCloseParams = decode(params)?;
        self.terminal_service_for(params.workspace_folder_id.as_deref())?
            .close(connection.connection_id, &params.terminal_id)
            .map_err(terminal_error)?;
        result(&())
    }
}

fn terminal_error(error: crate::terminal_service::TerminalError) -> RpcError {
    use crate::terminal_service::TerminalError;
    match error {
        TerminalError::InvalidInput => RpcError::new(-32602, AppServerErrorName::InvalidParams),
        TerminalError::NotFound => RpcError::new(-32061, AppServerErrorName::TerminalNotFound),
        TerminalError::NotOwner => RpcError::new(-32062, AppServerErrorName::TerminalNotOwner),
        TerminalError::AttachRejected => {
            RpcError::new(-32065, AppServerErrorName::TerminalAttachRejected)
        }
        TerminalError::Busy => RpcError::new(-32063, AppServerErrorName::TerminalBusy),
        TerminalError::OperationFailed => {
            RpcError::new(-32064, AppServerErrorName::TerminalOperationFailed)
        }
    }
}
