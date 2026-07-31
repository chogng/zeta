use super::ThreadTimeline;
use crate::shell_style::SHELL_PALETTE;
use crate::thread_projection::ThreadProjection;
use zeta_protocol::{
    ItemId, SessionId, Thread, ThreadId, ThreadItem, ThreadStatus, ToolCallId, ToolName, Turn,
    TurnId, TurnStatus,
};
use zeta_ui::Rect;

#[test]
fn timeline_groups_shell_result_under_its_tool_call() {
    let mut projection = ThreadProjection::default();
    projection.replace_snapshot(Thread {
        session_id: SessionId::new("session").unwrap(),
        thread_id: ThreadId::new("thread").unwrap(),
        title: "Agent".to_owned(),
        status: ThreadStatus::Active,
        sequence: 5,
        turns: vec![Turn {
            turn_id: TurnId::new("turn").unwrap(),
            status: TurnStatus::Completed,
            model: None,
            items: vec![
                ThreadItem::UserMessage {
                    item_id: ItemId::new("user").unwrap(),
                    turn_id: TurnId::new("turn").unwrap(),
                    text: "run the tests".to_owned(),
                },
                ThreadItem::ToolCall {
                    item_id: ItemId::new("call-item").unwrap(),
                    turn_id: TurnId::new("turn").unwrap(),
                    tool_call_id: ToolCallId::new("call").unwrap(),
                    name: ToolName::new("shell-command").unwrap(),
                    arguments_json: r#"{"program":"/bin/sh","arguments":["-lc","cargo test"],"working_directory":"."}"#.to_owned(),
                },
                ThreadItem::ToolResult {
                    item_id: ItemId::new("result").unwrap(),
                    turn_id: TurnId::new("turn").unwrap(),
                    tool_call_id: ToolCallId::new("call").unwrap(),
                    text: r#"{"exit_code":0,"stdout":"42 passed\n","stderr":"","stdout_truncated":false,"stderr_truncated":false}"#.to_owned(),
                    is_error: false,
                },
            ],
            pending_interaction: None,
            error: None,
        }],
    });

    let timeline = ThreadTimeline::new(
        Rect::from_xywh(0.0, 0.0, 800.0, 600.0),
        &projection,
        0,
        SHELL_PALETTE,
    );
    assert_eq!(
        timeline.visible_text(),
        vec![
            "You",
            "run the tests",
            "Tool · shell-command",
            "$ cargo test",
            "42 passed"
        ]
    );
}
