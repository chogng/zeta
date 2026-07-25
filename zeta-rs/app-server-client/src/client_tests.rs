use super::*;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_app_server_protocol::common::ClientInfo;
use zeta_app_server_protocol::v1::thread::ThreadReadParams;
use zeta_app_server_protocol::v1::thread::ThreadStartParams;
use zeta_app_server_protocol::v1::thread::TurnStatusDto;
use zeta_app_server_protocol::v1::turn::InputItem;
use zeta_app_server_protocol::v1::turn::InputItemKind;
use zeta_app_server_protocol::v1::turn::TurnStartParams;

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
    let state_root = temporary_directory("in-process");
    let mut client = start_in_process_client(InProcessClientOptions::new(
        &state_root,
        ClientInfo {
            name: "test-client".into(),
            version: "1".into(),
        },
    ))
    .expect("in-process client starts");
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

    drop(client);
    fs::remove_dir_all(state_root).expect("temporary state is removable");
}

fn temporary_directory(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock follows epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "zeta-app-server-client-{label}-{}-{nonce}",
        std::process::id()
    ))
}
