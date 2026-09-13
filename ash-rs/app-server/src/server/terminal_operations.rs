use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use ash_app_server_protocol::protocol::common::EmptyParams;
use ash_app_server_protocol::protocol::error::AppServerErrorName;
use ash_app_server_protocol::protocol::terminal::TerminalAttachParams;
use ash_app_server_protocol::protocol::terminal::TerminalCloseParams;
use ash_app_server_protocol::protocol::terminal::TerminalCreateInSessionDirectoryParams;
use ash_app_server_protocol::protocol::terminal::TerminalCreateParams;
use ash_app_server_protocol::protocol::terminal::TerminalProfileListResult;
use ash_app_server_protocol::protocol::terminal::TerminalReadParams;
use ash_app_server_protocol::protocol::terminal::TerminalResizeParams;
use ash_app_server_protocol::protocol::terminal::TerminalWriteParams;

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
            .terminal_service_for(params.dir_id.as_deref())?
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
        let authorization = self.session_dir_authorization(
            &params.session_id,
            &params.path,
            ash_file_access::Permission::ExecuteCommands,
        )?;
        let created = self
            .terminal_service()?
            .create_in_dir(
                connection.connection_id,
                TerminalCreateParams {
                    dir_id: None,
                    rows: params.rows,
                    cols: params.cols,
                    profile: params.profile,
                    lifecycle: params.lifecycle,
                },
                authorization,
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
        self.terminal_service_for(params.dir_id.as_deref())?
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
            .terminal_service_for(params.dir_id.as_deref())?
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
        self.terminal_service_for(params.dir_id.as_deref())?
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
            .terminal_service_for(params.dir_id.as_deref())?
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
        self.terminal_service_for(params.dir_id.as_deref())?
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
