use super::*;
use ash_action_policy::ActionDigest;
use ash_action_policy::ActionKind;
use ash_action_policy::ActionPolicyRevision;
use ash_action_policy::ActionProvenance;
use ash_action_policy::ActionSource;
use ash_action_policy::CapabilitySet;
use ash_action_policy::ResolvedAction;
use ash_action_policy::ReviewEvidenceKind;
use ash_action_policy::ReviewEvidenceTrust;
use ash_action_policy::SandboxCompatibility;
use ash_protocol::ItemId;
use ash_protocol::ThreadItem;
use ash_protocol::ToolCallId;
use ash_protocol::ToolName;
use ash_protocol::TurnId;

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
        ReviewEvidenceKind::DirectoryFile,
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
        ActionPolicyRevision::new("policy-1"),
    )
}
