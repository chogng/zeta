use crate::{
    ActionClassifier, ActionReviewPhase, ActionReviewRequest, ApprovalRequest, AutoReviewGrant,
    BlockReason, BuiltInSafetyPolicy, ClassifierAssessment, ClassifierRecommendation,
    ExecutionDecision, PolicyError, PolicyRevision, ReviewFailurePolicy, RiskLevel,
    SaferActionRequest, SandboxCompatibility, UserAllowlist, UserAuthorization,
};
use zeta_async_utils::CancellationToken;

/// Resolves exact rules, grants, sandbox compatibility, and advisory classifier output.
pub struct PolicyEngine<C> {
    revision: PolicyRevision,
    classifier: C,
    built_in_policy: BuiltInSafetyPolicy,
    user_allowlist: UserAllowlist,
    review_failure_policy: ReviewFailurePolicy,
}

impl<C: ActionClassifier> PolicyEngine<C> {
    pub fn new(
        revision: PolicyRevision,
        classifier: C,
        review_failure_policy: ReviewFailurePolicy,
    ) -> Self {
        Self {
            revision,
            classifier,
            built_in_policy: BuiltInSafetyPolicy::default(),
            user_allowlist: UserAllowlist::default(),
            review_failure_policy,
        }
    }

    /// Installs host-owned rules that take precedence over user authorization and model review.
    pub fn with_builtin_policy(mut self, policy: BuiltInSafetyPolicy) -> Self {
        self.built_in_policy = policy;
        self
    }

    /// Installs exact user-authorized actions that may run without sandbox enforcement.
    pub fn with_user_allowlist(mut self, allowlist: UserAllowlist) -> Self {
        self.user_allowlist = allowlist;
        self
    }

    pub fn revision(&self) -> &PolicyRevision {
        &self.revision
    }

    pub fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, PolicyError> {
        self.ensure_revision(request)?;
        if let Some(decision) = self.built_in_policy.decision(request) {
            return Ok(decision);
        }
        if let Some(grant) = self.user_allowlist.matching_grant(request) {
            return Ok(ExecutionDecision::RunUnsandboxed {
                grant_id: grant.id().clone(),
            });
        }
        if matches!(request.phase(), ActionReviewPhase::Initial)
            && let SandboxCompatibility::Supported(policy) = request.sandbox()
        {
            return Ok(ExecutionDecision::RunSandboxed(*policy));
        }

        let assessment = match self.classifier.classify(request, cancellation) {
            Ok(assessment) => assessment,
            Err(error) => return Ok(self.review_failure_decision(request, error.to_string())),
        };
        self.apply_assessment(request, assessment)
    }

    fn ensure_revision(&self, request: &ActionReviewRequest) -> Result<(), PolicyError> {
        if self.revision == *request.policy_revision() {
            Ok(())
        } else {
            Err(PolicyError::RevisionMismatch {
                engine: self.revision.clone(),
                request: request.policy_revision().clone(),
            })
        }
    }

    fn apply_assessment(
        &self,
        request: &ActionReviewRequest,
        assessment: ClassifierAssessment,
    ) -> Result<ExecutionDecision, PolicyError> {
        if assessment.action_digest() != request.action().digest()
            || assessment.policy_revision() != request.policy_revision()
        {
            return Err(PolicyError::ClassifierBindingMismatch);
        }
        if let Err(error) = assessment
            .recommendation()
            .validate_against(request.action().required_capabilities())
        {
            return Ok(ExecutionDecision::Block(BlockReason::ReviewFailed {
                reason: error.to_string(),
            }));
        }
        let decision = match assessment.recommendation() {
            ClassifierRecommendation::Approve {
                capabilities,
                risk,
                user_authorization,
                reason,
            } => self.automatic_approval_decision(
                request,
                &assessment,
                capabilities,
                *risk,
                *user_authorization,
                reason,
            ),
            ClassifierRecommendation::ReviseAction {
                maximum_capabilities,
                reason,
            } => ExecutionDecision::ReviseAction(SaferActionRequest::new(
                assessment.assessment_id().clone(),
                maximum_capabilities.clone(),
                reason,
            )),
            ClassifierRecommendation::AskUser { reason } => {
                ExecutionDecision::AskUser(ApprovalRequest::new(
                    request.action().digest().clone(),
                    request.action().required_capabilities().clone(),
                    reason,
                ))
            }
            ClassifierRecommendation::Deny { reason } => {
                ExecutionDecision::Block(BlockReason::ReviewerDenied {
                    assessment_id: assessment.assessment_id().clone(),
                    reason: reason.clone(),
                })
            }
        };
        Ok(decision)
    }

    fn automatic_approval_decision(
        &self,
        request: &ActionReviewRequest,
        assessment: &ClassifierAssessment,
        capabilities: &crate::CapabilitySet,
        risk: RiskLevel,
        user_authorization: UserAuthorization,
        reason: &str,
    ) -> ExecutionDecision {
        if risk == RiskLevel::Critical {
            return ExecutionDecision::Block(BlockReason::CriticalRisk {
                assessment_id: assessment.assessment_id().clone(),
                reason: reason.to_owned(),
            });
        }
        let authorized = matches!(
            (risk, user_authorization),
            (
                RiskLevel::Low | RiskLevel::Medium,
                UserAuthorization::Explicit | UserAuthorization::Implicit
            ) | (RiskLevel::High, UserAuthorization::Explicit)
        );
        if !authorized {
            return ExecutionDecision::AskUser(ApprovalRequest::new(
                request.action().digest().clone(),
                capabilities.clone(),
                format!("automatic review needs explicit user authorization: {reason}"),
            ));
        }
        ExecutionDecision::RunAutoReviewed(AutoReviewGrant::new(
            assessment.assessment_id().clone(),
            request.action().digest().clone(),
            capabilities.clone(),
            request.policy_revision().clone(),
        ))
    }

    fn review_failure_decision(
        &self,
        request: &ActionReviewRequest,
        reason: String,
    ) -> ExecutionDecision {
        match self.review_failure_policy {
            ReviewFailurePolicy::Block => {
                ExecutionDecision::Block(BlockReason::ReviewFailed { reason })
            }
            ReviewFailurePolicy::AskUser => ExecutionDecision::AskUser(ApprovalRequest::new(
                request.action().digest().clone(),
                request.action().required_capabilities().clone(),
                format!("automatic review was unavailable: {reason}"),
            )),
        }
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
