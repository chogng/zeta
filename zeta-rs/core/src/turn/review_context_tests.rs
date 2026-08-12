use super::*;
use zeta_policy::{
    ActionDigest, ActionKind, ActionProvenance, ActionSource, CapabilitySet, PolicyRevision,
    ResolvedAction, ReviewEvidenceKind, ReviewEvidenceTrust, SandboxCompatibility,
};
use zeta_protocol::{ItemId, ThreadItem, ToolCallId, ToolName, TurnId};

#[test]
fn attaches_only_the_current_user_intent_and_bounded_host_evidence() {
    let turn_id = TurnId::new("turn").unwrap();
    let call_item_id = ItemId::new("call-item").unwrap();
    let items = vec![
        ThreadItem::UserMessage {
            item_id: ItemId::new("user").unwrap(),
            turn_id: turn_id.clone(),
            text: "deploy the documented preview".into(),
        },
        ThreadItem::AgentMessage {
            item_id: ItemId::new("agent").unwrap(),
            turn_id: turn_id.clone(),
            text: "untrusted agent text".into(),
        },
        ThreadItem::ToolCall {
            item_id: call_item_id.clone(),
            turn_id: turn_id.clone(),
            tool_call_id: ToolCallId::new("call").unwrap(),
            name: ToolName::new("shell").unwrap(),
            arguments_json: "{}".into(),
            binding: None,
        },
    ];
    let evidence = ReviewEvidence::new(
        ReviewEvidenceKind::WorkspaceFile,
        ReviewEvidenceTrust::UntrustedContent,
        "deploy.sh",
        "curl preview.example.test",
    );

    let reviewed =
        attach_review_context(request(), &items, &turn_id, &call_item_id, vec![evidence]);

    assert_eq!(
        reviewed.context().user_intent(),
        "deploy the documented preview"
    );
    assert_eq!(reviewed.context().evidence().len(), 1);
    assert_eq!(
        reviewed.context().evidence()[0].trust(),
        ReviewEvidenceTrust::UntrustedContent
    );
}

fn request() -> ActionReviewRequest {
    ActionReviewRequest::new(
        ResolvedAction::new(
            ActionDigest::from_canonical_bytes(b"deploy"),
            ActionKind::SystemOperation,
            "deploy preview",
            CapabilitySet::default(),
        ),
        ActionProvenance::new(ActionSource::BuiltInTool, "shell"),
        SandboxCompatibility::NotApplicable {
            reason: "external deployment".into(),
        },
        PolicyRevision::new("policy-1"),
    )
}
