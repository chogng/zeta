use crate::client::new_command_id;
use crate::components::composer::ComposerInput;
use crate::components::composer::ComposerSubmission;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::thread::ThreadReadParams;
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_app_server_protocol::protocol::turn::TurnInterruptParams;
use zeta_app_server_protocol::protocol::turn::TurnInterruptResult;
use zeta_app_server_protocol::protocol::turn::TurnStartParams;
use zeta_app_server_protocol::protocol::turn::TurnStartResult;
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
}

pub(crate) fn submit_prompt<T>(
    client: &mut AppServerClient<T>,
    scope: ThreadRequestScope,
    submission: ComposerSubmission,
) -> Result<TurnStartResult, ClientError>
where
    T: JsonRpcTransport,
{
    client.start_turn(TurnStartParams {
        command_id: new_command_id("turn"),
        session_id: scope.session_id,
        thread_id: scope.thread_id,
        expected_sequence: scope.expected_sequence,
        input: submission
            .input
            .into_iter()
            .map(|input| match input {
                ComposerInput::Text(text) => InputItem::Text { text },
                ComposerInput::Image { url } => InputItem::Image { url },
            })
            .collect(),
    })
}

pub(crate) fn read_thread<T>(
    client: &mut AppServerClient<T>,
    thread_id: &ThreadId,
) -> Result<Thread, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_thread(ThreadReadParams {
            thread_id: thread_id.clone(),
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
    client.interrupt_turn(TurnInterruptParams {
        command_id: new_command_id("interrupt"),
        session_id: scope.session_id,
        thread_id: scope.thread_id,
        turn_id: turn_id.clone(),
        expected_sequence: scope.expected_sequence,
    })
}
