use crate::{DynamicToolResponse, RequestId, RequestUserInputResponse, TurnId, UserInput};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ThreadCommand {
    StartTurn {
        input: Vec<UserInput>,
    },
    InterruptTurn {
        turn_id: TurnId,
    },
    ResolveUserInput {
        turn_id: TurnId,
        request_id: RequestId,
        response: RequestUserInputResponse,
    },
    ResolveDynamicTool {
        turn_id: TurnId,
        request_id: RequestId,
        response: DynamicToolResponse,
    },
}
