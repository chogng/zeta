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
    GitUnavailable,
    GitNotRepository,
    GitOperationFailed,
    SearchUnavailable,
    SearchNotFound,
    SearchNotOwner,
    SearchBusy,
    TerminalUnavailable,
    TerminalNotFound,
    TerminalNotOwner,
    TerminalBusy,
    TerminalOperationFailed,
    ConfigUnavailable,
    ConfigRevisionConflict,
    SkillsUnavailable,
    SkillOperationFailed,
    SkillNotFound,
    WorkspaceSwitchUnavailable,
    WorkspaceSwitchBusy,
    WorkspaceSwitchFailed,
    WorkspaceTrustRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
pub struct AppServerError {
    #[ts(type = "number")]
    pub code: i64,
    pub message: AppServerErrorName,
    pub data: (),
}
