use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
pub enum AppServerErrorName {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerOverloaded,
    NotInitialized,
    AlreadyInitialized,
    CommandConflict,
    CoreOperationFailed,
    ResourceNotFound,
    ResourceNotOwner,
    ResourceTooLarge,
    InvalidResourceChunkSize,
    InvalidResourceOffset,
    FileSystemUnavailable,
    FileSystemOperationFailed,
    ConfigUnavailable,
    ConfigRevisionConflict,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
pub struct AppServerError {
    #[ts(type = "number")]
    pub code: i64,
    pub message: AppServerErrorName,
    pub data: (),
}
