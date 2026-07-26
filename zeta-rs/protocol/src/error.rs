use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StableTurnError {
    pub code: StableTurnErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl StableTurnError {
    pub fn model_invocation_failed() -> Self {
        Self {
            code: StableTurnErrorCode::ModelInvocationFailed,
            message: "Model invocation failed".into(),
            retryable: true,
        }
    }

    pub fn completion_persistence_failed() -> Self {
        Self {
            code: StableTurnErrorCode::CompletionPersistenceFailed,
            message: "Turn completion could not be persisted".into(),
            retryable: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StableTurnErrorCode {
    ModelInvocationFailed,
    CompletionPersistenceFailed,
}
