use super::request;
use crate::features::interactions::InteractionRequest;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::RequestId;
use zeta_protocol::TurnId;

#[test]
fn approval_request_keeps_fixed_capability_labels_and_structured_scope() {
    let interaction = request(
        TurnId::new("turn-1").unwrap(),
        RequestId::new("request-1").unwrap(),
        ActionApprovalRequest {
            action_digest: "sha256:test".into(),
            policy_revision: "1".into(),
            capabilities: vec![ActionApprovalCapability {
                kind: ActionApprovalCapabilityKind::ProcessSpawn,
                scope: "cargo test".into(),
            }],
            reason: "Run tests?".into(),
            sandbox_denial: None,
        },
    );

    let InteractionRequest::Approval { spec, .. } = interaction else {
        panic!("expected approval request");
    };
    assert_eq!(spec.reason, "Run tests?");
    assert_eq!(spec.details, ["Process spawn  ·  cargo test"]);
}
