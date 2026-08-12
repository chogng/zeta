mod active;
mod threads;
mod view;

pub(crate) use active::ActiveConversation;
pub(crate) use active::ConversationChange;
pub(crate) use active::ConversationTranscript;
pub(crate) use active::NewConversationKind;
pub(crate) use active::ResumeOutcome;
pub(crate) use threads::ThreadSelectionAction;
pub(crate) use threads::ThreadSelectionPurpose;
pub(crate) use threads::ThreadSelectionView;
pub(crate) use threads::thread_selection_view;
pub(crate) use view::SessionSelectionAction;
pub(crate) use view::SessionSelectionView;
pub(crate) use view::session_selection_view;

use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    active_session_id: &str,
) -> Result<SessionSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_sessions()
        .map(|result| session_selection_view(&result.sessions, active_session_id))
}

pub(crate) fn load_thread_selection<T>(
    client: &mut AppServerClient<T>,
    session_id: &zeta_protocol::SessionId,
    current_thread_id: &zeta_protocol::ThreadId,
    purpose: ThreadSelectionPurpose,
) -> Result<ThreadSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_session(
            zeta_app_server_protocol::protocol::session::SessionReadParams {
                session_id: session_id.clone(),
            },
        )
        .map(|result| thread_selection_view(&result.session, current_thread_id, purpose))
}
