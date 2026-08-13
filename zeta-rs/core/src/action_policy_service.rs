use crate::CoreError;
use zeta_action_policy::ActionClassifier;
use zeta_action_policy::ActionPolicyEngine;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ApprovalRequest;
use zeta_action_policy::CapabilityKind;
use zeta_action_policy::ExecutionDecision;
use zeta_action_policy::PermissionBypassGrant;
use zeta_async_utils::CancellationToken;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::ApprovalMode;
use zeta_protocol::SandboxDenialOutput;
use zeta_protocol::ToolReplaySafety;

/// Evaluates one fully resolved action without executing it or mutating durable Thread state.
///
/// Implementations must return only the final decision produced for the supplied immutable policy
/// request. `AskUser` remains a request for durable interaction; it is never authorization.
pub trait ActionPolicyService: Send + Sync {
    /// Returns the current immutable policy-environment revision at a Turn safe point.
    fn revision(&self) -> String;

    /// Evaluates under a Turn's durable policy ceiling.
    ///
    /// A changed policy environment fails closed by default. Implementations that can prove a
    /// newer revision is no wider may override this method and apply the stricter decision.
    fn decide_for_turn(
        &self,
        frozen_revision: &str,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        let current_revision = self.revision();
        if current_revision != frozen_revision {
            return Err(CoreError::Policy(format!(
                "Turn policy revision changed from {frozen_revision} to {current_revision}; continuation requires explicit authorization"
            )));
        }
        self.decide(request, cancellation)
    }

    /// Applies one Turn's frozen approval ceiling after the owning policy evaluates the action.
    ///
    /// Permission bypass converts only an interactive approval result. Deterministic blocks,
    /// invalid requests, revision mismatches, sandbox execution, and other policy decisions remain
    /// unchanged. Implementations with an automatic reviewer may override this method to replace
    /// `AskUser` in [`ApprovalMode::AutoReview`] while retaining the same fail-closed boundary.
    fn decide_for_turn_with_approval_mode(
        &self,
        frozen_revision: &str,
        approval_mode: ApprovalMode,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        let decision = self.decide_for_turn(frozen_revision, request, cancellation)?;
        if approval_mode == ApprovalMode::BypassPermissions
            && matches!(decision, ExecutionDecision::AskUser(_))
        {
            return Ok(ExecutionDecision::RunWithPermissionBypass(
                PermissionBypassGrant::new(
                    request.action().digest().clone(),
                    request.action().required_capabilities().clone(),
                    request.action_policy_revision().clone(),
                ),
            ));
        }
        Ok(decision)
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError>;
}

impl<C: ActionClassifier> ActionPolicyService for ActionPolicyEngine<C> {
    fn revision(&self) -> String {
        ActionPolicyEngine::revision(self).as_str().to_owned()
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        ActionPolicyEngine::decide(self, request, cancellation)
            .map_err(|error| CoreError::Policy(error.to_string()))
    }
}

pub(crate) struct UnavailableActionPolicyService;

impl ActionPolicyService for UnavailableActionPolicyService {
    fn revision(&self) -> String {
        "unavailable-policy-v1".into()
    }

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
    if reviewed.action_policy_revision().as_str().trim().is_empty() {
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
        policy_revision: reviewed.action_policy_revision().as_str().to_owned(),
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
        && approval.policy_revision == reviewed.action_policy_revision().as_str()
        && approval.capabilities
            == reviewed
                .action()
                .required_capabilities()
                .iter()
                .map(protocol_capability)
                .collect::<Vec<_>>()
}

fn protocol_capability(capability: &zeta_action_policy::Capability) -> ActionApprovalCapability {
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
#[path = "action_policy_service_tests.rs"]
mod tests;
