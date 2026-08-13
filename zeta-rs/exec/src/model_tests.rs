use super::EXEC_EVENT_SCHEMA_VERSION;
use super::ExecEntry;
use super::ExecEvent;
use super::ExecEventKind;
use super::ExecOrigin;
use super::ExecRunRequest;
use crate::ExecRunId;
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

#[test]
fn event_envelope_is_versioned_and_tagged() {
    let event = ExecEvent::new(
        &ExecRunId::new("run-contract").unwrap(),
        ExecEventKind::RunStarted {
            origin: ExecOrigin::Local,
            session_id: SessionId::new("session-1").unwrap(),
            thread_id: ThreadId::new("thread-1").unwrap(),
        },
    );
    let value = serde_json::to_value(event).unwrap();
    assert_eq!(value["schemaVersion"], EXEC_EVENT_SCHEMA_VERSION);
    assert_eq!(value["event"]["type"], "runStarted");
    assert_eq!(value["event"]["sessionId"], "session-1");
}

#[test]
fn run_request_round_trips_entry_intent() {
    let request = ExecRunRequest::new(ExecEntry::New {
        title: "contract".into(),
        input: vec![InputItem::Text {
            text: "inspect this".into(),
        }],
    })
    .with_run_id(ExecRunId::new("run-round-trip").unwrap());
    let encoded = serde_json::to_string(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<ExecRunRequest>(&encoded).unwrap(),
        request
    );
}
