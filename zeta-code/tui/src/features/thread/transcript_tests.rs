use super::CellLifecycle;
use super::TranscriptCellId;
use super::TranscriptProjection;
use crate::components::chat_history::CommandStatus;
use std::collections::BTreeSet;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptEntry;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::ToolOutputStream;
use zeta_protocol::TurnId;

#[test]
fn tool_call_output_and_result_form_one_exec_cell() {
    let turn_id = turn_id("turn");
    let tool_call_id = call_id("call");
    let mut projection = TranscriptProjection::default();
    projection.replace(snapshot(vec![
        ThreadTranscriptEntry::Item {
            entry_id: "call-entry".into(),
            turn_id: turn_id.clone(),
            item: ThreadItem::ToolCall {
                item_id: item_id("call-item"),
                turn_id: turn_id.clone(),
                tool_call_id: tool_call_id.clone(),
                name: ToolName::new("exec").unwrap(),
                arguments_json: "{\"cmd\":\"test\"}".into(),
                binding: None,
            },
            transient: false,
        },
        ThreadTranscriptEntry::ToolOutput {
            entry_id: "output-entry".into(),
            turn_id: turn_id.clone(),
            tool_call_id: tool_call_id.clone(),
            stream: ToolOutputStream::Stdout,
            text: "running".into(),
        },
        ThreadTranscriptEntry::Item {
            entry_id: "result-entry".into(),
            turn_id: turn_id.clone(),
            item: ThreadItem::ToolResult {
                item_id: item_id("result-item"),
                turn_id,
                tool_call_id,
                text: "passed".into(),
                content: None,
                is_error: false,
            },
            transient: false,
        },
    ]));

    assert_eq!(projection.cells().len(), 1);
    assert_eq!(projection.cells()[0].lifecycle(), CellLifecycle::Final);
    let views = projection.views(&BTreeSet::new(), None);
    assert_eq!(views[0].command_status, Some(CommandStatus::Succeeded));
    assert_eq!(views[0].text, "Ran exec");
}

#[test]
fn expansion_is_derived_without_changing_cell_lifecycle() {
    let turn_id = turn_id("turn");
    let tool_call_id = call_id("call");
    let mut projection = TranscriptProjection::default();
    projection.replace(snapshot(vec![ThreadTranscriptEntry::Item {
        entry_id: "call-entry".into(),
        turn_id: turn_id.clone(),
        item: ThreadItem::ToolCall {
            item_id: item_id("call-item"),
            turn_id,
            tool_call_id: tool_call_id.clone(),
            name: ToolName::new("exec").unwrap(),
            arguments_json: "{\"cmd\":\"test\"}".into(),
            binding: None,
        },
        transient: true,
    }]));
    let mut expanded = BTreeSet::new();
    let cell_id = TranscriptCellId::for_tool_call(&tool_call_id);
    expanded.insert(cell_id.clone());

    let view = projection.views(&expanded, Some(&cell_id));
    assert!(view[0].expanded);
    assert!(view[0].selected);
    assert_eq!(projection.cells()[0].lifecycle(), CellLifecycle::Live);
}

#[test]
fn exec_cell_identity_is_derived_from_the_first_tool_call_across_resync() {
    let turn_id = turn_id("turn");
    let call_id = call_id("stable-call");
    let entries = vec![ThreadTranscriptEntry::Item {
        entry_id: "replaceable-entry".into(),
        turn_id: turn_id.clone(),
        item: ThreadItem::ToolCall {
            item_id: item_id("call-item"),
            turn_id,
            tool_call_id: call_id.clone(),
            name: ToolName::new("exec").unwrap(),
            arguments_json: "{}".into(),
            binding: None,
        },
        transient: false,
    }];
    let mut projection = TranscriptProjection::default();
    projection.replace(snapshot(entries.clone()));
    let first = projection.cells()[0].cell_id().clone();
    projection.replace(snapshot(entries));

    assert_eq!(first, TranscriptCellId::for_tool_call(&call_id));
    assert_eq!(projection.cells()[0].cell_id(), &first);
}

#[test]
fn reinstalling_a_cell_advances_its_render_revision() {
    let turn_id = turn_id("turn");
    let entries = vec![ThreadTranscriptEntry::Item {
        entry_id: "agent-entry".into(),
        turn_id: turn_id.clone(),
        item: ThreadItem::AgentMessage {
            item_id: item_id("agent-item"),
            turn_id,
            text: "streamed answer".into(),
        },
        transient: false,
    }];
    let mut projection = TranscriptProjection::default();
    projection.replace(snapshot(entries.clone()));
    let first = projection.views(&BTreeSet::new(), None)[0].render_revision;

    projection.replace(snapshot(entries));
    let second = projection.views(&BTreeSet::new(), None)[0].render_revision;

    assert!(second > first);
}

fn snapshot(entries: Vec<ThreadTranscriptEntry>) -> ThreadTranscriptSnapshot {
    ThreadTranscriptSnapshot {
        session_id: session_id("session"),
        thread_id: thread_id("thread"),
        durable_sequence: 1,
        revision: 1,
        entries,
    }
}

fn turn_id(value: &str) -> TurnId {
    TurnId::new(value).expect("the test Turn ID is valid")
}

fn call_id(value: &str) -> ToolCallId {
    ToolCallId::new(value).expect("the test ToolCall ID is valid")
}

fn item_id(value: &str) -> ItemId {
    ItemId::new(value).expect("the test item ID is valid")
}

fn session_id(value: &str) -> SessionId {
    SessionId::new(value).expect("the test Session ID is valid")
}

fn thread_id(value: &str) -> ThreadId {
    ThreadId::new(value).expect("the test Thread ID is valid")
}
