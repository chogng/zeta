use super::RewindPaneSpec;
use super::rewind_pane_spec;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

pub(crate) fn load_selection<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    thread_id: &ThreadId,
) -> Result<RewindPaneSpec, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_session_thread(SessionThreadReadParams {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            history: None,
        })
        .map(|result| rewind_pane_spec(&result.thread))
}
