mod active;
mod manager;
mod pane;
mod state;

pub(crate) use active::ActiveConversation;
pub(crate) use active::ConversationChange;
pub(crate) use active::ConversationTranscript;
pub(crate) use active::NewConversationKind;
pub(crate) use active::ResumeOutcome;
pub(crate) use manager::SessionManagerView;
pub(crate) use manager::draw_manager;
pub(crate) use pane::SessionPaneSpec;
pub(crate) use pane::SessionSelectionAction;
pub(crate) use pane::session_pane_spec;
pub(crate) use state::RootTarget;
pub(crate) use state::SessionsState;

use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::SessionRequest;
use zeta_app_server_protocol::protocol::session::SessionRequestParams;
use zeta_protocol::Session;
use zeta_protocol::SessionId;

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

pub(crate) fn load_catalog<T>(
    client: &mut AppServerClient<T>,
) -> Result<Vec<zeta_protocol::Session>, ClientError>
where
    T: JsonRpcTransport,
{
    client.list_sessions().map(|result| result.sessions)
}

pub(crate) fn archive<T>(
    client: &mut AppServerClient<T>,
    session_ids: Vec<SessionId>,
) -> Result<Vec<Session>, ClientError>
where
    T: JsonRpcTransport,
{
    for session_id in session_ids {
        client
            .request_session(SessionRequestParams {
                command_id: crate::client::new_command_id("archive"),
                session_id,
                request: SessionRequest::Archive,
            })
            .and_then(active::expect_session_result)?;
    }
    load_catalog(client)
}
