use super::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use zeta_app_server::AppServer;
use zeta_app_server_protocol::protocol::common::{ClientCapabilities, ClientInfo};
use zeta_app_server_protocol::protocol::initialize::InitializeParams;
use zeta_app_server_protocol::protocol::session::{SessionCreateParams, SessionThreadCreateParams};
use zeta_app_server_protocol::protocol::thread::ThreadReadParams;
use zeta_app_server_protocol::protocol::turn::{InputItem, InputItemKind, TurnStartParams};
use zeta_app_server_protocol::schema_hash;
use zeta_async_utils::CancellationToken;
use zeta_core::{
    CoreError, InMemorySessionStore, InMemoryThreadStore, ModelService, SessionCoordinator,
    ThreadController,
};
use zeta_protocol::{
    CommandId, ContentPart, InputItem as ModelInputItem, ModelRequest, ModelResponse, ResponseItem,
    StopReason, ThreadEvent, ThreadItem, ThreadUpdate, TurnStatus,
};

struct MockTransport(VecDeque<String>);

impl JsonRpcTransport for MockTransport {
    fn round_trip(&mut self, _: &str) -> Result<String, ClientError> {
        self.0
            .pop_front()
            .ok_or_else(|| ClientError::Transport("no response".into()))
    }
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

#[test]
fn client_rejects_response_for_another_request() {
    let mut client = AppServerClient::new(MockTransport(VecDeque::from([
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}".into(),
    ])));
    let result: Result<serde_json::Value, _> =
        client.call(ClientMethod::Initialize, serde_json::json!({}));
    assert!(matches!(result, Err(ClientError::Protocol(_))));
}

#[test]
fn in_process_client_uses_session_first_contract_and_canonical_updates() {
    let mut client = AppServerClient::new(InProcessTransport::from_server(app_server()));
    let initialized = client
        .initialize(InitializeParams {
            client_info: ClientInfo {
                name: "test-client".into(),
                version: "1".into(),
            },
            capabilities: ClientCapabilities::default(),
        })
        .expect("in-process client initializes");
    assert_eq!(initialized.schema_hash.0, schema_hash());
    let session = client
        .create_session(SessionCreateParams {
            command_id: CommandId::new("session-one").expect("test ID is non-empty"),
            title: "test".into(),
        })
        .expect("Session is created");
    let thread = client
        .create_session_thread(SessionThreadCreateParams {
            command_id: CommandId::new("thread-one").expect("test ID is non-empty"),
            session_id: session.session.session_id.clone(),
            expected_sequence: session.session.sequence,
            title: "root".into(),
        })
        .expect("Thread is created");
    let turn = client
        .start_turn(TurnStartParams {
            command_id: CommandId::new("turn-one").expect("test ID is non-empty"),
            session_id: session.session.session_id,
            thread_id: thread.thread_id.clone(),
            expected_sequence: 1,
            input: vec![InputItem {
                kind: InputItemKind::Text,
                text: "hello".into(),
            }],
        })
        .expect("Turn starts");
    let deadline = Instant::now() + Duration::from_secs(1);
    let snapshot = loop {
        let snapshot = client
            .read_thread(ThreadReadParams {
                thread_id: thread.thread_id.clone(),
            })
            .expect("Thread remains readable");
        if snapshot.thread.turns[0].status == TurnStatus::Completed {
            break snapshot;
        }
        assert!(Instant::now() < deadline, "Turn did not complete");
        thread::sleep(Duration::from_millis(1));
    };
    let notifications = client.drain_notifications().expect("notifications decode");
    let output = notifications
        .iter()
        .find_map(|notification| match notification {
            ServerNotification::ThreadUpdate(update) => match &update.update {
                ThreadUpdate::Committed {
                    event:
                        ThreadEvent::ItemCompleted {
                            item: ThreadItem::AgentMessage { text, .. },
                            ..
                        },
                } => Some(text.as_str()),
                _ => None,
            },
            _ => None,
        });
    assert_eq!(output, Some("Zeta: hello"));
    assert_eq!(snapshot.thread.turns[0].turn_id, turn.turn_id);
    assert_eq!(snapshot.thread.turns[0].status, TurnStatus::Completed);
}
