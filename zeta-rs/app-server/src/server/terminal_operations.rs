use super::{AppServer, ConnectionState, RpcError, decode, result};
use serde_json::Value;
use zeta_app_server_protocol::protocol::common::EmptyParams;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::terminal::{
    TerminalCloseParams, TerminalCreateParams, TerminalProfileListResult, TerminalReadParams,
    TerminalResizeParams, TerminalWriteParams,
};

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
            .terminal_service()?
            .create(connection.connection_id, params)
            .map_err(terminal_error)?;
        result(&created)
    }

    pub(super) fn terminal_write(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalWriteParams = decode(params)?;
        self.terminal_service()?
            .write(connection.connection_id, params)
            .map_err(terminal_error)?;
        result(&())
    }

    pub(super) fn terminal_resize(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TerminalResizeParams = decode(params)?;
        self.terminal_service()?
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
            .terminal_service()?
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
        self.terminal_service()?
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
        TerminalError::Busy => RpcError::new(-32063, AppServerErrorName::TerminalBusy),
        TerminalError::OperationFailed => {
            RpcError::new(-32064, AppServerErrorName::TerminalOperationFailed)
        }
    }
}
