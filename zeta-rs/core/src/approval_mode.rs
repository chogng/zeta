//! Per-Turn approval-mode enforcement; extensions provide advice, never grants.
use crate::ActionPolicyService;
use crate::CoreError;
use std::sync::Arc;
use zeta_action_policy::ActionPolicyEngine;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ExecutionDecision;
use zeta_action_policy::PermissionBypassGrant;
use zeta_action_policy::ReviewFailurePolicy;
use zeta_async_utils::CancellationToken;
use zeta_protocol::ApprovalMode;
/// Adds per-Turn approval-mode semantics around one authoritative Tool policy.
///
/// The base policy always evaluates first. Automatic review and permission bypass may replace
/// only an `AskUser` result, so deterministic denial and all request/revision validation stay
/// owned by the underlying policy.
pub struct ApprovalModeActionPolicyService {
    base: Arc<dyn ActionPolicyService>,
    reviewer: zeta_extension_api::ApprovalReviewer,
}

impl ApprovalModeActionPolicyService {
    pub fn new(
        base: Arc<dyn ActionPolicyService>,
        reviewer: zeta_extension_api::ApprovalReviewer,
    ) -> Self {
        Self { base, reviewer }
    }
}

impl ActionPolicyService for ApprovalModeActionPolicyService {
    fn revision(&self) -> String {
        let reviewer = match &self.reviewer {
            zeta_extension_api::ApprovalReviewer::Unavailable => "unavailable",
            zeta_extension_api::ApprovalReviewer::Configured { identity, .. } => identity.as_str(),
        };
        format!(
            "{}:auto-review={reviewer}:isolated-controls=v1",
            self.base.revision()
        )
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        if request.provenance().source() == &zeta_action_policy::ActionSource::BuiltInTool
            && request.provenance().source_id() == "code-mode"
        {
            cancellation
                .check()
                .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
            let capabilities =
                zeta_action_policy::CapabilitySet::new([zeta_action_policy::Capability::new(
                    zeta_action_policy::CapabilityKind::SystemConfiguration,
                    "code-mode",
                )]);
            if request.action().kind() != &zeta_action_policy::ActionKind::SystemOperation
                || request.action().required_capabilities() != &capabilities
                || request.action_policy_revision().as_str() != self.revision()
                || !matches!(
                    request.phase(),
                    zeta_action_policy::ActionReviewPhase::Initial
                )
                || !matches!(
                    request.sandbox(),
                    zeta_action_policy::SandboxCompatibility::NotApplicable { .. }
                )
            {
                return Err(CoreError::Policy(
                    "Code Mode control request exceeds its isolated runtime authority".into(),
                ));
            }
            // The runtime has no ambient host authority. Every nested tool re-enters its own policy.
            return Ok(zeta_action_policy::ExecutionDecision::RunUnsandboxed {
                grant_id: zeta_action_policy::GrantId::new("host-code-mode-control"),
            });
        }
        self.base.decide(request, cancellation)
    }

    fn decide_for_turn_with_approval_mode(
        &self,
        frozen_revision: &str,
        approval_mode: ApprovalMode,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        let current_revision = self.revision();
        if current_revision != frozen_revision {
            return Err(CoreError::Policy(format!(
                "Turn policy revision changed from {frozen_revision} to {current_revision}; continuation requires explicit authorization"
            )));
        }
        let decision = self.decide(request, cancellation)?;
        if !matches!(decision, ExecutionDecision::AskUser(_)) {
            return Ok(decision);
        }
        match approval_mode {
            ApprovalMode::AskPermissions => Ok(decision),
            ApprovalMode::BypassPermissions => Ok(ExecutionDecision::RunWithPermissionBypass(
                PermissionBypassGrant::new(
                    request.action().digest().clone(),
                    request.action().required_capabilities().clone(),
                    request.action_policy_revision().clone(),
                ),
            )),
            ApprovalMode::AutoReview => {
                let zeta_extension_api::ApprovalReviewer::Configured { registry, .. } =
                    &self.reviewer
                else {
                    return Ok(decision);
                };
                let engine = ActionPolicyEngine::with_no_exec_rules(
                    request.action_policy_revision().clone(),
                    RegistryReviewer(registry.clone()),
                    ReviewFailurePolicy::AskUser,
                );
                let reviewed = engine
                    .review_after_authoritative_ask_user(request, cancellation)
                    .map_err(|error| CoreError::Policy(error.to_string()))?;
                cancellation
                    .check()
                    .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
                Ok(reviewed)
            }
        }
    }
}

struct RegistryReviewer(Arc<zeta_extension_api::ExtensionRegistry>);
impl zeta_action_policy::ActionClassifier for RegistryReviewer {
    type Error = zeta_extension_api::ExtensionError;
    fn classify(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<zeta_action_policy::ClassifierAssessment, Self::Error> {
        self.0.review(request, cancellation)
    }
}
