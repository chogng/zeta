use crate::{
    ActionApprovalResponse, DynamicToolResponse, ModelRef, RequestId, RequestUserInputResponse,
    TurnId, UserInput,
};
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        model: Option<ModelRef>,
        input: Vec<UserInput>,
    },
    InterruptTurn {
        turn_id: TurnId,
    },
    ResolveApproval {
        turn_id: TurnId,
        request_id: RequestId,
        response: ActionApprovalResponse,
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
