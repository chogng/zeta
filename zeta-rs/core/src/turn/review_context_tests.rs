use super::*;
use zeta_action_policy::ActionDigest;
use zeta_action_policy::ActionKind;
use zeta_action_policy::ActionPolicyRevision;
use zeta_action_policy::ActionProvenance;
use zeta_action_policy::ActionSource;
use zeta_action_policy::CapabilitySet;
use zeta_action_policy::ResolvedAction;
use zeta_action_policy::ReviewEvidenceKind;
use zeta_action_policy::ReviewEvidenceTrust;
use zeta_action_policy::SandboxCompatibility;
use zeta_protocol::ItemId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;
use zeta_protocol::TurnId;

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
        ActionPolicyRevision::new("policy-1"),
    )
}
