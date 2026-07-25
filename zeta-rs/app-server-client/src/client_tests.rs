use super::*;
use std::collections::VecDeque;
use std::sync::Arc;
use zeta_app_server::AppServer;
use zeta_app_server_protocol::CURRENT_PROTOCOL_VERSION;
use zeta_app_server_protocol::common::ClientInfo;
use zeta_app_server_protocol::common::{ClientCapabilities, ProtocolVersions};
use zeta_app_server_protocol::schema_hash_v1;
use zeta_app_server_protocol::v1::initialize::InitializeParams;
use zeta_app_server_protocol::v1::thread::ThreadReadParams;
use zeta_app_server_protocol::v1::thread::ThreadStartParams;
use zeta_app_server_protocol::v1::thread::TurnStatusDto;
use zeta_app_server_protocol::v1::turn::InputItem;
use zeta_app_server_protocol::v1::turn::InputItemKind;
use zeta_app_server_protocol::v1::turn::TurnStartParams;
use zeta_core::{InMemoryJournal, ThreadManager};
use zeta_model_provider::EchoModel;

struct MockTransport(VecDeque<String>);
impl JsonRpcTransport for MockTransport {
    fn round_trip(&mut self, _: &str) -> Result<String, ClientError> {
        self.0
            .pop_front()
            .ok_or_else(|| ClientError::Transport("no response".into()))
    }
}

#[test]
fn client_rejects_response_for_another_request() {
    let mut client = AppServerClient::new(MockTransport(VecDeque::from([
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}".into(),
    ])));
    let result: Result<serde_json::Value, _> = client.call("test", serde_json::json!({}));
    assert!(matches!(result, Err(ClientError::Protocol(_))));
}

#[test]
fn in_process_client_uses_the_same_protocol_and_notifications_as_external_clients() {
    let mut client = AppServerClient::new(InProcessTransport::from_server(AppServer::new(
        Arc::new(ThreadManager::with_journal(Arc::new(
            InMemoryJournal::default(),
        ))),
        Arc::new(EchoModel),
    )));
    let initialized = client
        .initialize(InitializeParams {
            client_info: ClientInfo {
                name: "test-client".into(),
                version: "1".into(),
            },
            protocol_versions: ProtocolVersions {
                min: CURRENT_PROTOCOL_VERSION,
                max: CURRENT_PROTOCOL_VERSION,
            },
            capabilities: ClientCapabilities::default(),
        })
        .expect("in-process client initializes");
    assert_eq!(initialized.schema_hash.0, schema_hash_v1());
    let thread = client
        .start_thread(ThreadStartParams {
            idempotency_key: "thread-one".into(),
            title: "test".into(),
        })
        .expect("thread starts");
    let turn = client
        .start_turn(TurnStartParams {
            idempotency_key: "turn-one".into(),
            thread_id: thread.thread_id.clone(),
            input: vec![InputItem {
                kind: InputItemKind::Text,
                text: "hello".into(),
            }],
        })
        .expect("turn starts");
    let notifications = client.drain_notifications().expect("notifications decode");
    let output = notifications
        .iter()
        .find_map(|notification| match notification {
            ServerNotification::AgentMessageCompleted(message) => Some(message.text.as_str()),
            _ => None,
        });
    let snapshot = client
        .read_thread(ThreadReadParams {
            thread_id: thread.thread_id,
        })
        .expect("thread remains readable");

    assert_eq!(output, Some("Zeta: hello"));
    assert!(notifications.iter().any(|notification| {
        matches!(
            notification,
            ServerNotification::TurnCompleted(completed) if completed.turn_id == turn.turn_id
        )
    }));
    assert_eq!(snapshot.thread.turns[0].status, TurnStatusDto::Completed);
}
