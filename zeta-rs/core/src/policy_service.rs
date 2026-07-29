use crate::CoreError;
use zeta_async_utils::CancellationToken;
use zeta_policy::{
    ActionClassifier, ActionReviewRequest, ApprovalRequest, CapabilityKind, ExecutionDecision,
    PolicyEngine,
};
use zeta_protocol::{
    ActionApprovalCapability, ActionApprovalCapabilityKind, ActionApprovalRequest,
    SandboxDenialOutput, ToolReplaySafety,
};

/// Evaluates one fully resolved action without executing it or mutating durable Thread state.
///
/// Implementations must return only the final decision produced for the supplied immutable policy
/// request. `AskUser` remains a request for durable interaction; it is never authorization.
pub trait PolicyService: Send + Sync {
    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError>;
}

impl<C: ActionClassifier> PolicyService for PolicyEngine<C> {
    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        PolicyEngine::decide(self, request, cancellation)
            .map_err(|error| CoreError::Policy(error.to_string()))
    }
}

pub(crate) struct UnavailablePolicyService;

impl PolicyService for UnavailablePolicyService {
    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Err(CoreError::Policy(
            "no action policy service is configured".into(),
        ))
    }
}

/// Converts a policy `AskUser` decision into an exact durable interaction payload.
///
/// The policy revision comes from the reviewed request rather than the client. The approval must
/// remain bound to the same digest and complete capability set or conversion fails closed.
pub fn durable_approval_request(
    reviewed: &ActionReviewRequest,
    approval: &ApprovalRequest,
) -> Result<ActionApprovalRequest, CoreError> {
    if approval.action_digest() != reviewed.action().digest()
        || approval.capabilities() != reviewed.action().required_capabilities()
    {
        return Err(CoreError::Policy(
            "approval request is not bound to the reviewed action".into(),
        ));
    }
    if approval.reason().trim().is_empty() {
        return Err(CoreError::Policy(
            "approval request reason must not be empty".into(),
        ));
    }
    if reviewed.policy_revision().as_str().trim().is_empty() {
        return Err(CoreError::Policy(
            "approval policy revision must not be empty".into(),
        ));
    }
    let capabilities = approval
        .capabilities()
        .iter()
        .map(protocol_capability)
        .collect::<Vec<_>>();
    if capabilities.is_empty() {
        return Err(CoreError::Policy(
            "approval request must contain at least one capability".into(),
        ));
    }
    if capabilities
        .iter()
        .any(|capability| capability.scope.trim().is_empty())
    {
        return Err(CoreError::Policy(
            "approval capability scope must not be empty".into(),
        ));
    }
    Ok(ActionApprovalRequest {
        action_digest: reviewed.action().digest().as_str().to_owned(),
        policy_revision: reviewed.policy_revision().as_str().to_owned(),
        capabilities,
        reason: approval.reason().to_owned(),
        sandbox_denial: None,
    })
}

pub(crate) fn durable_sandbox_escalation_approval_request(
    reviewed: &ActionReviewRequest,
    approval: &ApprovalRequest,
    denial: SandboxDenialOutput,
) -> Result<ActionApprovalRequest, CoreError> {
    if denial.replay_safety() != ToolReplaySafety::SafeToRetry || denial.reason().trim().is_empty()
    {
        return Err(CoreError::Policy(
            "sandbox escalation approval requires a safe-to-retry denial".into(),
        ));
    }
    let mut request = durable_approval_request(reviewed, approval)?;
    request.sandbox_denial = Some(denial);
    Ok(request)
}

pub(crate) fn approval_matches_review(
    approval: &ActionApprovalRequest,
    reviewed: &ActionReviewRequest,
) -> bool {
    approval.action_digest == reviewed.action().digest().as_str()
        && approval.policy_revision == reviewed.policy_revision().as_str()
        && approval.capabilities
            == reviewed
                .action()
                .required_capabilities()
                .iter()
                .map(protocol_capability)
                .collect::<Vec<_>>()
}

fn protocol_capability(capability: &zeta_policy::Capability) -> ActionApprovalCapability {
    ActionApprovalCapability {
        kind: match capability.kind() {
            CapabilityKind::FileRead => ActionApprovalCapabilityKind::FileRead,
            CapabilityKind::FileWrite => ActionApprovalCapabilityKind::FileWrite,
            CapabilityKind::ProcessSpawn => ActionApprovalCapabilityKind::ProcessSpawn,
            CapabilityKind::Network => ActionApprovalCapabilityKind::Network,
            CapabilityKind::CredentialUse => ActionApprovalCapabilityKind::CredentialUse,
            CapabilityKind::ExternalMutation => ActionApprovalCapabilityKind::ExternalMutation,
            CapabilityKind::SystemConfiguration => {
                ActionApprovalCapabilityKind::SystemConfiguration
            }
            CapabilityKind::UserInterface => ActionApprovalCapabilityKind::UserInterface,
        },
        scope: capability.scope().to_owned(),
    }
}

#[cfg(test)]
#[path = "policy_service_tests.rs"]
mod tests;
