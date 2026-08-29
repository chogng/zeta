use super::InteractionBinding;
use super::InteractionRequest;
use crate::components::approval::ApprovalSpec;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::RequestId;
use zeta_protocol::TurnId;

pub(super) fn request(
    turn_id: TurnId,
    request_id: RequestId,
    request: ActionApprovalRequest,
) -> InteractionRequest {
    InteractionRequest::Approval {
        binding: InteractionBinding::Approval {
            turn_id,
            request_id,
        },
        spec: ApprovalSpec {
            title: "Approval required".into(),
            reason: request.reason,
            details: request.capabilities.iter().map(capability_detail).collect(),
        },
    }
}

fn capability_detail(capability: &ActionApprovalCapability) -> String {
    format!(
        "{}  ·  {}",
        capability_kind(capability.kind),
        capability.scope
    )
}

fn capability_kind(kind: ActionApprovalCapabilityKind) -> &'static str {
    match kind {
        ActionApprovalCapabilityKind::FileRead => "File read",
        ActionApprovalCapabilityKind::FileWrite => "File write",
        ActionApprovalCapabilityKind::ProcessSpawn => "Process spawn",
        ActionApprovalCapabilityKind::Network => "Network",
        ActionApprovalCapabilityKind::CredentialUse => "Credential use",
        ActionApprovalCapabilityKind::ExternalMutation => "External mutation",
        ActionApprovalCapabilityKind::SystemConfiguration => "System configuration",
        ActionApprovalCapabilityKind::UserInterface => "User interface",
    }
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;
