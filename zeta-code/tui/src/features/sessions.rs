mod active;
mod pane;

pub(crate) use active::ActiveConversation;
pub(crate) use active::ConversationChange;
pub(crate) use active::ConversationTranscript;
pub(crate) use active::NewConversationKind;
pub(crate) use active::ResumeOutcome;
pub(crate) use pane::SessionPaneSpec;
pub(crate) use pane::SessionSelectionAction;
pub(crate) use pane::session_pane_spec;

use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    active_session_id: &str,
) -> Result<SessionPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .list_sessions()
        .map(|result| session_pane_spec(&result.sessions, active_session_id))
}
