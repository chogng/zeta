mod active;
mod view;

pub(crate) use active::ActiveConversation;
pub(crate) use active::ConversationChange;
pub(crate) use active::ConversationTranscript;
pub(crate) use active::NewConversationKind;
pub(crate) use active::ResumeOutcome;
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
