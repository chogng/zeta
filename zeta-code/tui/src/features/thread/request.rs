use crate::client::new_command_id;
use crate::components::composer::ComposerInput;
use crate::components::composer::ComposerSubmission;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_app_server_protocol::protocol::session::{
    SessionRequest, SessionRequestParams, SessionRequestResult, SessionThreadReadParams,
};
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_app_server_protocol::protocol::turn::TurnInteractionResolveResult;
use zeta_app_server_protocol::protocol::turn::TurnInterruptResult;
use zeta_app_server_protocol::protocol::turn::TurnStartResult;
use zeta_protocol::AgentResponse;
use zeta_protocol::RequestId;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

/// Identifies the aggregate and canonical sequence used by one typed Thread write.
pub(crate) struct ThreadRequestScope {
    session_id: SessionId,
    thread_id: ThreadId,
    expected_sequence: u64,
}

impl ThreadRequestScope {
    pub(crate) fn new(
        session_id: &SessionId,
        thread_id: &ThreadId,
        expected_sequence: u64,
    ) -> Self {
        Self {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            expected_sequence,
        }
    }

    pub(crate) fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub(crate) fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }
}

pub(crate) fn submit_prompt<T>(
    client: &mut AppServerClient<T>,
    scope: ThreadRequestScope,
    submission: ComposerSubmission,
) -> Result<TurnStartResult, ClientError>
where
    T: JsonRpcTransport,
{
    match client.request_session(SessionRequestParams {
        command_id: new_command_id("turn"),
        session_id: scope.session_id,
        expected_sequence: scope.expected_sequence,
        request: SessionRequest::StartTurn {
            thread_id: scope.thread_id,
            input: submission
                .input
                .into_iter()
                .map(|input| match input {
                    ComposerInput::Text(text) => InputItem::Text { text },
                    ComposerInput::Image { url } => InputItem::Image { url },
                    ComposerInput::Skill { skill } => InputItem::Skill { skill },
                })
                .collect(),
        },
    })? {
        SessionRequestResult::Turn(result) => Ok(result),
        other => Err(ClientError::Protocol(format!(
            "session request returned {other:?} for StartTurn"
        ))),
    }
}

#[cfg(test)]
pub(crate) fn read_thread<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    thread_id: &ThreadId,
) -> Result<Thread, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_session_thread(SessionThreadReadParams {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            history: None,
        })
        .map(|result| result.thread)
}

pub(crate) fn read_thread_history<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    thread_id: &ThreadId,
    history: ThreadSnapshotHistory,
) -> Result<Thread, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_session_thread(SessionThreadReadParams {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            history: Some(history),
        })
        .map(|result| result.thread)
}

pub(crate) fn interrupt_turn<T>(
    client: &mut AppServerClient<T>,
    scope: ThreadRequestScope,
    turn_id: &TurnId,
) -> Result<TurnInterruptResult, ClientError>
where
    T: JsonRpcTransport,
{
    match client.request_session(SessionRequestParams {
        command_id: new_command_id("interrupt"),
        session_id: scope.session_id,
        expected_sequence: scope.expected_sequence,
        request: SessionRequest::InterruptTurn {
            thread_id: scope.thread_id,
            turn_id: turn_id.clone(),
        },
    })? {
        SessionRequestResult::TurnInterrupt(result) => Ok(result),
        other => Err(ClientError::Protocol(format!(
            "session request returned {other:?} for InterruptTurn"
        ))),
    }
}

pub(crate) fn resolve_interaction<T>(
    client: &mut AppServerClient<T>,
    scope: ThreadRequestScope,
    turn_id: TurnId,
    request_id: RequestId,
    response: AgentResponse,
) -> Result<TurnInteractionResolveResult, ClientError>
where
    T: JsonRpcTransport,
{
    match client.request_session(SessionRequestParams {
        command_id: new_command_id("interaction"),
        session_id: scope.session_id,
        expected_sequence: scope.expected_sequence,
        request: SessionRequest::ResolveInteraction {
            thread_id: scope.thread_id,
            turn_id,
            request_id,
            response,
        },
    })? {
        SessionRequestResult::Interaction(result) => Ok(result),
        other => Err(ClientError::Protocol(format!(
            "session request returned {other:?} for ResolveInteraction"
        ))),
    }
}
