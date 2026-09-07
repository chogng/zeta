use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use std::time::Duration;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::memory::MemorySessionParams;
use zeta_app_server_protocol::protocol::resources::ResourceMetadataResult;
use zeta_memory_diagnostics::MemoryDiagnosticsError;
use zeta_memory_diagnostics::MemoryEvidence;
use zeta_memory_diagnostics::MemoryStart;

impl AppServer {
    pub(super) fn memory_start(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let state = connection
            .state
            .lock()
            .map_err(|_| RpcError::new(-32603, AppServerErrorName::InternalError))?;
        if state.closed {
            return Err(RpcError::new(-32800, AppServerErrorName::RequestCancelled));
        }
        let params: MemoryStart = decode(params)?;
        result(
            &self
                .memory
                .start(connection.connection_id, params)
                .map_err(error)?,
        )
    }

    pub(super) fn memory_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let state = connection
            .state
            .lock()
            .map_err(|_| RpcError::new(-32603, AppServerErrorName::InternalError))?;
        if state.closed {
            return Err(RpcError::new(-32800, AppServerErrorName::RequestCancelled));
        }
        let params: MemorySessionParams = decode(params)?;
        result(
            &self
                .memory
                .read(connection.connection_id, &params.session_id)
                .map_err(error)?,
        )
    }

    pub(super) fn memory_stop(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let state = connection
            .state
            .lock()
            .map_err(|_| RpcError::new(-32603, AppServerErrorName::InternalError))?;
        if state.closed {
            return Err(RpcError::new(-32800, AppServerErrorName::RequestCancelled));
        }
        let params: MemorySessionParams = decode(params)?;
        result(
            &self
                .memory
                .stop(connection.connection_id, &params.session_id)
                .map_err(error)?,
        )
    }

    pub(super) fn memory_submit(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let state = connection
            .state
            .lock()
            .map_err(|_| RpcError::new(-32603, AppServerErrorName::InternalError))?;
        if state.closed {
            return Err(RpcError::new(-32800, AppServerErrorName::RequestCancelled));
        }
        let params: MemoryEvidence = decode(params)?;
        self.memory
            .submit(connection.connection_id, params)
            .map_err(error)?;
        result(&())
    }

    pub(super) fn memory_export(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let state = connection
            .state
            .lock()
            .map_err(|_| RpcError::new(-32603, AppServerErrorName::InternalError))?;
        if state.closed {
            return Err(RpcError::new(-32800, AppServerErrorName::RequestCancelled));
        }
        let params: MemorySessionParams = decode(params)?;
        let bytes = self
            .memory
            .export(connection.connection_id, &params.session_id)
            .map_err(error)?;
        let metadata = self
            .resources
            .lock()
            .map_err(|_| RpcError::new(-32603, AppServerErrorName::InternalError))?
            .create(
                connection.connection_id,
                "application/json".into(),
                bytes,
                Duration::from_secs(600),
            )
            .map_err(|_| RpcError::new(-32112, AppServerErrorName::MemoryCapacity))?;
        result(&ResourceMetadataResult {
            resource_id: metadata.resource_id,
            mime_type: metadata.mime_type,
            size: metadata.size,
            sha256: metadata.sha256,
        })
    }
}

fn error(error: MemoryDiagnosticsError) -> RpcError {
    let name = match error {
        MemoryDiagnosticsError::Invalid => AppServerErrorName::InvalidParams,
        MemoryDiagnosticsError::Capacity => AppServerErrorName::MemoryCapacity,
        MemoryDiagnosticsError::NotFound => AppServerErrorName::MemoryNotFound,
        MemoryDiagnosticsError::Conflict => AppServerErrorName::MemoryConflict,
        MemoryDiagnosticsError::Stopped => AppServerErrorName::MemoryStopped,
        MemoryDiagnosticsError::Stale => AppServerErrorName::MemoryStale,
        MemoryDiagnosticsError::Unavailable => AppServerErrorName::MemoryUnavailable,
    };
    RpcError::new(
        if name == AppServerErrorName::InvalidParams {
            -32602
        } else {
            -32110
        },
        name,
    )
}
