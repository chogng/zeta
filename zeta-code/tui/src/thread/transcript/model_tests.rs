use super::CellLifecycle;
use super::TranscriptCellId;
use super::TranscriptModel;
use crate::thread::transcript::CommandStatus;
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
    let mut model = TranscriptModel::default();
    model.replace(snapshot(vec![
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

    assert_eq!(model.cells().len(), 1);
    assert_eq!(model.cells()[0].lifecycle(), CellLifecycle::Final);
    let views = model.views(&BTreeSet::new(), None);
    assert_eq!(views[0].command_status(), Some(CommandStatus::Succeeded));
    assert_eq!(views[0].text(), "Ran exec");
}

#[test]
fn expansion_is_derived_without_changing_cell_lifecycle() {
    let turn_id = turn_id("turn");
    let tool_call_id = call_id("call");
    let mut model = TranscriptModel::default();
    model.replace(snapshot(vec![ThreadTranscriptEntry::Item {
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

    let view = model.views(&expanded, Some(&cell_id));
    assert!(view[0].expanded);
    assert!(view[0].selected);
    assert_eq!(model.cells()[0].lifecycle(), CellLifecycle::Live);
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
    let mut model = TranscriptModel::default();
    model.replace(snapshot(entries.clone()));
    let first = model.cells()[0].cell_id().clone();
    model.replace(snapshot(entries));

    assert_eq!(first, TranscriptCellId::for_tool_call(&call_id));
    assert_eq!(model.cells()[0].cell_id(), &first);
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
    let mut model = TranscriptModel::default();
    model.replace(snapshot(entries.clone()));
    let first = model.views(&BTreeSet::new(), None)[0].render_revision;

    model.replace(snapshot(entries));
    let second = model.views(&BTreeSet::new(), None)[0].render_revision;

    assert!(second > first);
}

#[test]
fn first_live_cell_splits_committed_history_from_the_rendered_tail() {
    let turn_id = turn_id("turn");
    let call_id = call_id("call");
    let mut model = TranscriptModel::default();
    model.replace(snapshot(vec![
        ThreadTranscriptEntry::Item {
            entry_id: "user-entry".into(),
            turn_id: turn_id.clone(),
            item: ThreadItem::UserMessage {
                item_id: item_id("user-item"),
                turn_id: turn_id.clone(),
                text: "committed prompt".into(),
            },
            transient: false,
        },
        ThreadTranscriptEntry::Item {
            entry_id: "call-entry".into(),
            turn_id: turn_id.clone(),
            item: ThreadItem::ToolCall {
                item_id: item_id("call-item"),
                turn_id: turn_id.clone(),
                tool_call_id: call_id,
                name: zeta_protocol::ToolName::new("exec").unwrap(),
                arguments_json: "{}".into(),
                binding: None,
            },
            transient: true,
        },
        ThreadTranscriptEntry::Item {
            entry_id: "agent-entry".into(),
            turn_id: turn_id.clone(),
            item: ThreadItem::AgentMessage {
                item_id: item_id("agent-item"),
                turn_id,
                text: "ordered behind live work".into(),
            },
            transient: false,
        },
    ]));

    assert_eq!(model.committed_cells(None).len(), 1);
    assert_eq!(model.active_cells(None).len(), 2);
    assert_eq!(
        model
            .active_views(None, &BTreeSet::new(), None)
            .last()
            .unwrap()
            .text(),
        "ordered behind live work"
    );
}

#[test]
fn a_completed_execution_group_stays_live_until_the_turn_closes() {
    let turn = turn_id("group-turn");
    let mut model = TranscriptModel::default();
    model.replace(snapshot(vec![
        tool_call("one", &turn),
        tool_result("one", &turn),
    ]));
    assert!(model.committed_cells(Some(&turn)).is_empty());
    assert_eq!(model.active_cells(Some(&turn)).len(), 1);
    let id = model.cells()[0].cell_id().clone();
    model.upsert(tool_call("two", &turn));
    model.upsert(tool_result("two", &turn));
    assert_eq!(model.cells().len(), 1);
    assert_eq!(model.cells()[0].cell_id(), &id);
    assert!(model.committed_cells(Some(&turn)).is_empty());
    let completed = model.committed_cells(None);
    assert_eq!(completed.len(), 1);
    let detail = completed[0].history_view().detail().unwrap().into_owned();
    assert!(detail.contains("result one"));
    assert!(detail.contains("result two"));
}

#[test]
fn execution_groups_never_merge_across_turns() {
    let first = turn_id("first");
    let second = turn_id("second");
    let mut model = TranscriptModel::default();
    model.replace(snapshot(vec![
        tool_call("one", &first),
        tool_result("one", &first),
        tool_call("two", &second),
    ]));
    assert_eq!(model.cells().len(), 2);
    assert_eq!(model.committed_cells(Some(&second)).len(), 1);
    assert_eq!(model.active_cells(Some(&second)).len(), 1);
    assert_eq!(
        model.cells()[1].cell_id(),
        &TranscriptCellId::for_tool_call(&call_id("two"))
    );
}

fn tool_call(name: &str, turn: &TurnId) -> ThreadTranscriptEntry {
    ThreadTranscriptEntry::Item {
        entry_id: format!("call-{name}"),
        turn_id: turn.clone(),
        transient: false,
        item: ThreadItem::ToolCall {
            item_id: item_id(&format!("item-{name}")),
            turn_id: turn.clone(),
            tool_call_id: call_id(name),
            name: ToolName::new("read_file").unwrap(),
            arguments_json: "{}".into(),
            binding: None,
        },
    }
}

fn tool_result(name: &str, turn: &TurnId) -> ThreadTranscriptEntry {
    ThreadTranscriptEntry::Item {
        entry_id: format!("result-{name}"),
        turn_id: turn.clone(),
        transient: false,
        item: ThreadItem::ToolResult {
            item_id: item_id(&format!("result-item-{name}")),
            turn_id: turn.clone(),
            tool_call_id: call_id(name),
            text: format!("result {name}"),
            content: None,
            is_error: false,
        },
    }
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
