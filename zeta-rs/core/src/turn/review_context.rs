use zeta_policy::{ActionReviewRequest, ReviewContext, ReviewEvidence};
use zeta_protocol::{ItemId, ThreadItem, TurnId};

const MAX_USER_INTENT_CHARS: usize = 4_000;
const MAX_EVIDENCE_ITEMS: usize = 8;
const MAX_EVIDENCE_CHARS: usize = 2_000;

pub(super) fn attach_review_context(
    request: ActionReviewRequest,
    items: &[ThreadItem],
    turn_id: &TurnId,
    pending_item_id: &ItemId,
    host_evidence: Vec<ReviewEvidence>,
) -> ActionReviewRequest {
    let user_intent = items
        .iter()
        .take_while(|item| item.item_id() != pending_item_id)
        .filter_map(|item| match item {
            ThreadItem::UserMessage {
                turn_id: item_turn_id,
                text,
                ..
            } if item_turn_id == turn_id => Some(truncate(text, MAX_USER_INTENT_CHARS)),
            _ => None,
        })
        .last()
        .unwrap_or_default();
    let evidence = host_evidence
        .into_iter()
        .take(MAX_EVIDENCE_ITEMS)
        .map(|item| {
            ReviewEvidence::new(
                item.kind(),
                item.trust(),
                truncate(item.source(), MAX_EVIDENCE_CHARS),
                truncate(item.content(), MAX_EVIDENCE_CHARS),
            )
        })
        .collect::<Vec<_>>();
    request.with_context(ReviewContext::new(user_intent, evidence))
}

fn truncate(value: &str, maximum_chars: usize) -> String {
    value.chars().take(maximum_chars).collect()
}

#[cfg(test)]
#[path = "review_context_tests.rs"]
mod tests;
