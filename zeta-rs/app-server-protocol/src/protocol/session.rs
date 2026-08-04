use crate::protocol::common::CommandId;
use crate::protocol::common::RequestId;
use crate::protocol::common::SessionId;
use crate::protocol::common::ThreadId;
use crate::protocol::common::TurnId;
use crate::protocol::turn::InputItem;
use crate::protocol::turn::TurnInteractionResolveResult;
use crate::protocol::turn::TurnInterruptResult;
use crate::protocol::turn::TurnStartResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeta_protocol::AgentResponse;
use zeta_protocol::ModelRef;
use zeta_protocol::Session;
use zeta_protocol::SessionUpdateEnvelope;
use zeta_protocol::Thread;
use zeta_protocol::ThreadUpdateEnvelope;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateParams {
    pub command_id: CommandId,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionReadParams {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionSubscribeParams {
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub after_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionUnsubscribeParams {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionCommandParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
}

/// A typed mutation submitted against one Session aggregate.
///
/// Child Thread identifiers are selectors inside the Session boundary. Product hosts submit
/// these operations through `session/request`; App Server owns the internal Thread routing.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionRequest {
    Complete,
    Archive,
    Stop,
    SetModel {
        model: ModelRef,
    },
    CreateThread {
        title: String,
    },
    ForkThread {
        parent_thread_id: ThreadId,
        title: String,
    },
    RewindThread {
        parent_thread_id: ThreadId,
        before_turn_id: TurnId,
        title: String,
    },
    ArchiveThread {
        thread_id: ThreadId,
    },
    StartTurn {
        thread_id: ThreadId,
        #[schemars(length(min = 1))]
        input: Vec<InputItem>,
    },
    StartShellTurn {
        thread_id: ThreadId,
        command: String,
        working_directory: String,
    },
    InterruptTurn {
        thread_id: ThreadId,
        turn_id: TurnId,
    },
    ResolveInteraction {
        thread_id: ThreadId,
        turn_id: TurnId,
        request_id: RequestId,
        response: AgentResponse,
    },
}

/// The canonical Session aggregate mutation request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionRequestParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub request: SessionRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionModelSetParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub model: ModelRef,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadCreateParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadForkParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub parent_thread_id: ThreadId,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadRewindParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub parent_thread_id: ThreadId,
    pub before_turn_id: TurnId,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadArchiveParams {
    pub command_id: CommandId,
    pub session_id: SessionId,
    #[ts(type = "number")]
    pub expected_sequence: u64,
    pub thread_id: ThreadId,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionResult {
    pub session: Session,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResult {
    pub sessions: Vec<Session>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
/// A child Thread snapshot and its committed gap carried by the Session subscription.
pub struct SessionThreadProjection {
    pub thread: Thread,
    pub updates: Vec<ThreadUpdateEnvelope>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
/// The aggregate Session port returned to product hosts.
pub struct SessionSubscribeResult {
    pub session: Session,
    pub updates: Vec<SessionUpdateEnvelope>,
    pub thread_projections: Vec<SessionThreadProjection>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionThreadResult {
    pub session: Session,
    pub thread_id: ThreadId,
}

/// Typed result returned by the canonical Session aggregate mutation endpoint.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum SessionRequestResult {
    Session(SessionResult),
    Thread(SessionThreadResult),
    Turn(TurnStartResult),
    TurnInterrupt(TurnInterruptResult),
    Interaction(TurnInteractionResolveResult),
}
