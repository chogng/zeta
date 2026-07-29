use super::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
use zeta_app_server_protocol::protocol::common::{ClientCapabilities, ClientInfo};
use zeta_app_server_protocol::protocol::session::{SessionCreateParams, SessionThreadCreateParams};
use zeta_app_server_protocol::protocol::thread::ThreadSubscribeParams;
use zeta_app_server_protocol::protocol::turn::{InputItem, TurnStartParams};
use zeta_async_utils::CancellationToken;
use zeta_core::{
    CoreError, InMemorySessionStore, InMemoryThreadStore, ModelService, SessionCoordinator,
    ThreadController,
};
use zeta_protocol::{
    CommandId, ContentPart, InputItem as ModelInputItem, ModelRequest, ModelResponse, ResponseItem,
    StopReason, ThreadEvent, ThreadUpdate,
};

#[test]
fn embedded_session_delivers_idle_notifications_without_a_polling_request() {
    let server = Arc::new(app_server());
    let mut session = AppServerSession::from_embedded_host(
        server,
        ClientInfo {
            name: "session-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let events = session.take_events().unwrap();
    let mut client = session.client();
    let created_session = client
        .create_session(SessionCreateParams {
            command_id: command_id("session"),
            title: "session".into(),
        })
        .unwrap();
    let created_thread = client
        .create_session_thread(SessionThreadCreateParams {
            command_id: command_id("thread"),
            session_id: created_session.session.session_id.clone(),
            expected_sequence: created_session.session.sequence,
            title: "thread".into(),
        })
        .unwrap();
    client
        .subscribe_thread(ThreadSubscribeParams {
            thread_id: created_thread.thread_id.clone(),
            after_sequence: 0,
        })
        .unwrap();
    client
        .start_turn(TurnStartParams {
            command_id: command_id("turn"),
            session_id: created_session.session.session_id,
            thread_id: created_thread.thread_id,
            expected_sequence: 1,
            input: vec![InputItem::Text {
                text: "hello".into(),
            }],
        })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    let completed = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "completion notification timed out");
        let event = events.recv_timeout(remaining).expect("event arrives");
        if let AppServerEvent::Notification(ServerNotification::ThreadUpdate(update)) = event
            && matches!(
                update.update,
                ThreadUpdate::Committed {
                    event: ThreadEvent::TurnCompleted { .. }
                }
            )
        {
            break true;
        }
    };
    assert!(completed);

    session.shutdown().unwrap();
    assert_eq!(
        events.recv_timeout(Duration::from_secs(1)).unwrap(),
        AppServerEvent::ConnectionClosed(ConnectionCloseReason::Shutdown)
    );
}

#[test]
fn shutdown_rejects_requests_from_surviving_client_clones() {
    let mut session = AppServerSession::from_embedded_host(
        Arc::new(app_server()),
        ClientInfo {
            name: "shutdown-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let events = session.take_events().unwrap();
    let mut client = session.client();

    session.shutdown().unwrap();

    assert!(matches!(
        client.list_sessions(),
        Err(ClientError::Transport(message)) if message.contains("closed")
    ));
    assert_eq!(
        events.recv_timeout(Duration::from_secs(1)).unwrap(),
        AppServerEvent::ConnectionClosed(ConnectionCloseReason::Shutdown)
    );
}

#[test]
fn request_handle_clones_share_initialization_and_request_ids() {
    let session = AppServerSession::from_embedded_host(
        Arc::new(app_server()),
        ClientInfo {
            name: "clone-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let mut first = session.client();
    let mut second = session.client();

    assert_eq!(
        first.initialization().unwrap(),
        second.initialization().unwrap()
    );
    first.list_sessions().unwrap();
    second.list_sessions().unwrap();

    session.shutdown().unwrap();
}

fn command_id(label: &str) -> CommandId {
    CommandId::new(format!("{label}-command")).unwrap()
}

fn app_server() -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    AppServer::new(
        Arc::new(SessionCoordinator::with_store(
            Arc::new(InMemorySessionStore::default()),
            threads,
        )),
        Arc::new(TestModel),
    )
}

struct TestModel;

impl ModelService for TestModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let prompt = request
            .input
            .iter()
            .rev()
            .find_map(|item| match item {
                ModelInputItem::Message(message) => {
                    message.content.iter().find_map(|content| match content {
                        ContentPart::Text(text) => Some(text.as_str()),
                        ContentPart::ImageUrl { .. } => None,
                    })
                }
                ModelInputItem::ToolResult(_) => None,
            })
            .unwrap_or_default();
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(format!("Zeta: {prompt}"))],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}
