use super::{AppServer, ConnectionState, RpcError, decode, result};
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::search::{
    WorkspaceSearchCancelParams, WorkspaceSearchReadParams, WorkspaceSearchStartParams,
    WorkspaceSearchStartResult,
};

impl AppServer {
    pub(super) fn workspace_search_start(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceSearchStartParams = decode(params)?;
        let search_id = self
            .workspace_search_service()?
            .start(connection.connection_id, params)
            .map_err(search_error)?;
        result(&WorkspaceSearchStartResult { search_id })
    }

    pub(super) fn workspace_search_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceSearchReadParams = decode(params)?;
        let search = self
            .workspace_search_service()?
            .read(connection.connection_id, params)
            .map_err(search_error)?;
        result(&search)
    }

    pub(super) fn workspace_search_cancel(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkspaceSearchCancelParams = decode(params)?;
        self.workspace_search_service()?
            .cancel(connection.connection_id, &params.search_id)
            .map_err(search_error)?;
        result(&())
    }
}

fn search_error(error: crate::workspace_search::WorkspaceSearchError) -> RpcError {
    use crate::workspace_search::WorkspaceSearchError;
    match error {
        WorkspaceSearchError::InvalidInput => {
            RpcError::new(-32602, AppServerErrorName::InvalidParams)
        }
        WorkspaceSearchError::NotFound => RpcError::new(-32051, AppServerErrorName::SearchNotFound),
        WorkspaceSearchError::NotOwner => RpcError::new(-32052, AppServerErrorName::SearchNotOwner),
        WorkspaceSearchError::Busy => RpcError::new(-32053, AppServerErrorName::SearchBusy),
    }
}
