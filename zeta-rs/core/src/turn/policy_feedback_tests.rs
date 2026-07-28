use super::*;
use zeta_policy::{AssessmentId, BlockReason};
use zeta_protocol::{ItemId, ThreadItem, ToolCallId, TurnId};

#[test]
fn opens_after_three_consecutive_reviewer_rejections() {
    let turn_id = TurnId::new("turn").unwrap();
    let text = denied_feedback(&BlockReason::ReviewerDenied {
        assessment_id: AssessmentId::new("assessment"),
        reason: "unsafe".into(),
    })
    .unwrap();
    let items = (0..3)
        .map(|index| ThreadItem::ToolResult {
            item_id: ItemId::new(format!("item-{index}")).unwrap(),
            turn_id: turn_id.clone(),
            tool_call_id: ToolCallId::new(format!("call-{index}")).unwrap(),
            text: text.clone(),
            is_error: true,
        })
        .collect::<Vec<_>>();

    assert!(rejection_circuit_breaker(&items, &turn_id).is_some());
}

#[test]
fn ordinary_tool_result_resets_the_consecutive_counter() {
    let turn_id = TurnId::new("turn").unwrap();
    let rejected = denied_feedback(&BlockReason::ReviewerDenied {
        assessment_id: AssessmentId::new("assessment"),
        reason: "unsafe".into(),
    })
    .unwrap();
    let result = |index: usize, text: String| ThreadItem::ToolResult {
        item_id: ItemId::new(format!("item-{index}")).unwrap(),
        turn_id: turn_id.clone(),
        tool_call_id: ToolCallId::new(format!("call-{index}")).unwrap(),
        text,
        is_error: true,
    };
    let items = vec![
        result(0, rejected.clone()),
        result(1, rejected.clone()),
        result(2, "ordinary failure".into()),
        result(3, rejected),
    ];

    assert!(rejection_circuit_breaker(&items, &turn_id).is_none());
}
