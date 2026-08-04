use super::RewindSelectionView;
use super::rewind_selection_view;
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
) -> Result<RewindSelectionView, ClientError>
where
    T: JsonRpcTransport,
{
    client
        .read_session_thread(SessionThreadReadParams {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
        })
        .map(|result| rewind_selection_view(&result.thread))
}
