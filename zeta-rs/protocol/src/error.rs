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

    pub fn context_overflow() -> Self {
        Self {
            code: StableTurnErrorCode::ContextOverflow,
            message: "The model context window was exceeded".into(),
            retryable: true,
        }
    }

    pub fn provider_auth() -> Self {
        Self {
            code: StableTurnErrorCode::ProviderAuth,
            message: "Model provider authentication failed".into(),
            retryable: false,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: StableTurnErrorCode::InvalidRequest,
            message: "The model rejected an invalid request".into(),
            retryable: false,
        }
    }

    pub fn invalid_response() -> Self {
        Self {
            code: StableTurnErrorCode::InvalidResponse,
            message: "The model returned an invalid response".into(),
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

    pub fn interaction_deadline_elapsed() -> Self {
        Self {
            code: StableTurnErrorCode::InteractionDeadlineElapsed,
            message: "Interaction deadline elapsed before a response was received".into(),
            retryable: true,
        }
    }

    pub fn tool_repetition() -> Self {
        Self {
            code: StableTurnErrorCode::ToolRepetition,
            message: "The same failing tool call was repeated too many times".into(),
            retryable: false,
        }
    }

    pub fn turn_budget_exhausted() -> Self {
        Self {
            code: StableTurnErrorCode::TurnBudgetExhausted,
            message: "The Turn resource budget was exhausted".into(),
            retryable: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StableTurnErrorCode {
    ModelInvocationFailed,
    ContextOverflow,
    ProviderAuth,
    InvalidRequest,
    InvalidResponse,
    CompletionPersistenceFailed,
    InteractionDeadlineElapsed,
    ToolRepetition,
    TurnBudgetExhausted,
}
