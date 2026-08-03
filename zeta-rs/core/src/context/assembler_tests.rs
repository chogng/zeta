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

    let request =
        ContextAssembler::assemble(&snapshot, Vec::new(), &HarnessInstructions::default()).unwrap();

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
fn groups_ordered_text_and_images_from_one_user_turn() {
    let turn_id = id::<TurnId>("turn");
    let image_url = "data:image/png;base64,iVBORw0KGgpwYXlsb2Fk";
    let snapshot = snapshot(
        turn_id.clone(),
        vec![
            ThreadItem::UserMessage {
                item_id: id("text-before"),
                turn_id: turn_id.clone(),
                text: "describe".into(),
            },
            ThreadItem::UserImage {
                item_id: id("image"),
                turn_id: turn_id.clone(),
                url: image_url.into(),
            },
            ThreadItem::UserMessage {
                item_id: id("text-after"),
                turn_id,
                text: "briefly".into(),
            },
        ],
    );

    let request =
        ContextAssembler::assemble(&snapshot, Vec::new(), &HarnessInstructions::default()).unwrap();

    assert_eq!(request.input.len(), 1);
    let InputItem::Message(message) = &request.input[0] else {
        panic!("user input must assemble as one message");
    };
    assert_eq!(message.role, MessageRole::User);
    assert_eq!(
        message.content,
        vec![
            ContentPart::Text("describe".into()),
            ContentPart::ImageUrl {
                url: image_url.into(),
                detail: ImageDetail::Auto,
            },
            ContentPart::Text("briefly".into()),
        ]
    );
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
        ContextAssembler::assemble(&snapshot, Vec::new(), &HarnessInstructions::default()),
        Err(CoreError::Context(_))
    ));
}

#[test]
fn injects_instructions_and_workspace_message_before_durable_history() {
    let turn_id = id::<TurnId>("turn");
    let snapshot = snapshot(
        turn_id.clone(),
        vec![ThreadItem::UserMessage {
            item_id: id("user"),
            turn_id,
            text: "hello".into(),
        }],
    );
    let instructions = HarnessInstructions::new(
        "system body",
        "<environment>\ntoday: 2026-08-03\n</environment>",
        Some("follow the workspace rules".into()),
    );

    let request = ContextAssembler::assemble(&snapshot, Vec::new(), &instructions).unwrap();

    assert_eq!(
        request.instructions.as_deref(),
        Some("system body\n\n<environment>\ntoday: 2026-08-03\n</environment>")
    );
    assert!(request.parallel_tool_calls);
    let InputItem::Message(message) = &request.input[0] else {
        panic!("workspace instructions must be the first input message");
    };
    assert!(matches!(message.role, MessageRole::User));
    assert!(
        matches!(&message.content[0], ContentPart::Text(text) if text.contains("Workspace instructions from AGENTS.md"))
    );
}

#[test]
fn repeated_assembly_is_byte_stable() {
    let turn_id = id::<TurnId>("turn");
    let snapshot = snapshot(
        turn_id.clone(),
        vec![ThreadItem::UserMessage {
            item_id: id("user"),
            turn_id,
            text: "stable".into(),
        }],
    );
    let instructions = HarnessInstructions::new("system", "environment", None);
    let first = ContextAssembler::assemble(&snapshot, Vec::new(), &instructions).unwrap();
    let second = ContextAssembler::assemble(&snapshot, Vec::new(), &instructions).unwrap();

    assert_eq!(
        format!("{first:?}").as_bytes(),
        format!("{second:?}").as_bytes()
    );
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
            model: None,
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
