use super::*;
use crate::{ThreadCommandSnapshot, TurnSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use zeta_protocol::{ItemId, SessionId, ThreadId, ToolCallId, ToolName, TurnId, TurnStatus};

#[test]
fn assembles_messages_and_paired_tool_results_from_durable_items() {
    let turn_id = id::<TurnId>("turn");
    let call_id = id::<ToolCallId>("call");
    let snapshot = snapshot(
        turn_id.clone(),
        vec![
            ThreadItem::UserMessage {
                item_id: id("user"),
                turn_id: turn_id.clone(),
                text: "weather?".into(),
            },
            ThreadItem::ToolCall {
                item_id: id("tool"),
                turn_id: turn_id.clone(),
                tool_call_id: call_id.clone(),
                name: ToolName::new("weather").unwrap(),
                arguments_json: r#"{"city":"Paris"}"#.into(),
            },
            ThreadItem::ToolResult {
                item_id: id("result"),
                turn_id,
                tool_call_id: call_id.clone(),
                text: "sunny".into(),
                is_error: false,
            },
        ],
    );

    let request = ContextAssembler::assemble(&snapshot, Vec::new()).unwrap();

    assert_eq!(request.input.len(), 3);
    let InputItem::Message(message) = &request.input[1] else {
        panic!("second input must be an assistant Tool Call");
    };
    assert_eq!(message.tool_calls[0].id, call_id);
    assert_eq!(message.tool_calls[0].arguments["city"], "Paris");
    let InputItem::ToolResult(result) = &request.input[2] else {
        panic!("third input must be a Tool Result");
    };
    assert_eq!(result.name.as_str(), "weather");
    assert_eq!(request.tool_choice, ToolChoice::None);
}

#[test]
fn rejects_invalid_durable_tool_arguments() {
    let turn_id = id::<TurnId>("turn");
    let snapshot = snapshot(
        turn_id.clone(),
        vec![ThreadItem::ToolCall {
            item_id: id("tool"),
            turn_id,
            tool_call_id: id("call"),
            name: ToolName::new("weather").unwrap(),
            arguments_json: "{".into(),
        }],
    );

    assert!(matches!(
        ContextAssembler::assemble(&snapshot, Vec::new()),
        Err(CoreError::Context(_))
    ));
}

fn snapshot(turn_id: TurnId, items: Vec<ThreadItem>) -> ThreadSnapshot {
    ThreadSnapshot {
        session_id: id::<SessionId>("session"),
        thread_id: id::<ThreadId>("thread"),
        title: "test".into(),
        sequence: items.len() as u64 + 2,
        turns: vec![TurnSnapshot {
            turn_id,
            status: TurnStatus::Running,
            failure: None,
            pending_interaction: None,
        }],
        items,
        commands: Vec::<ThreadCommandSnapshot>::new(),
        seen_interaction_ids: BTreeSet::new(),
        resolved_interactions: Vec::new(),
        started_tool_calls: BTreeSet::new(),
        tool_execution_starts: BTreeMap::new(),
        escalated_tool_calls: BTreeSet::new(),
    }
}

trait TestId: Sized {
    fn from_test(value: &str) -> Self;
}

macro_rules! impl_test_id {
    ($($type:ty),+ $(,)?) => {
        $(
            impl TestId for $type {
                fn from_test(value: &str) -> Self {
                    Self::new(value).expect("test ID is non-empty")
                }
            }
        )+
    };
}

impl_test_id!(ItemId, SessionId, ThreadId, ToolCallId, TurnId);

fn id<T: TestId>(value: &str) -> T {
    T::from_test(value)
}
