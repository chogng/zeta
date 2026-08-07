use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::collaboration::DocumentCollaborationOpenParams;
use zeta_app_server_protocol::protocol::collaboration::DocumentCollaborationSubmitParams;
use zeta_app_server_protocol::protocol::collaboration::DocumentCollaborationSubmitResult;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;

impl AppServer {
    pub(super) fn document_collaboration_open(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: DocumentCollaborationOpenParams = decode(params)?;
        let opened = self
            .collaboration
            .lock()
            .map_err(|_| collaboration_error())?
            .open(params)
            .map_err(invalid_collaboration_params)?;
        self.updates.subscribe_document_collaboration(
            connection.connection_id,
            opened.snapshot.room_id.clone(),
        );
        result(&opened)
    }

    pub(super) fn document_collaboration_submit(&self, params: &Value) -> Result<Value, RpcError> {
        let params: DocumentCollaborationSubmitParams = decode(params)?;
        let submitted = self
            .collaboration
            .lock()
            .map_err(|_| collaboration_error())?
            .submit(params)
            .map_err(invalid_collaboration_params)?;
        if let DocumentCollaborationSubmitResult::Accepted { update } = &submitted {
            self.updates.publish_document_collaboration(update.clone());
        }
        result(&submitted)
    }
}

fn invalid_collaboration_params(_error: String) -> RpcError {
    RpcError::new(-32602, AppServerErrorName::InvalidParams)
}

fn collaboration_error() -> RpcError {
    RpcError::new(-32603, AppServerErrorName::InternalError)
}
