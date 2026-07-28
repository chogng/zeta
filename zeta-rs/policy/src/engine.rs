use crate::{
    ActionClassifier, ActionReviewPhase, ActionReviewRequest, ActionRule, ApprovalRequest,
    AutoReviewGrant, BlockReason, ClassifierAssessment, ClassifierRecommendation,
    ExecutionDecision, PolicyError, PolicyRevision, ReviewFailurePolicy, RiskLevel, RuleEffect,
    SaferActionRequest, SandboxCompatibility, UnsandboxedGrant, UserAuthorization,
};
use zeta_async_utils::CancellationToken;

/// Resolves exact rules, grants, sandbox compatibility, and advisory classifier output.
pub struct PolicyEngine<C> {
    revision: PolicyRevision,
    classifier: C,
    rules: Vec<ActionRule>,
    grants: Vec<UnsandboxedGrant>,
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
            rules: Vec::new(),
            grants: Vec::new(),
            review_failure_policy,
        }
    }

    pub fn with_rules(mut self, rules: impl IntoIterator<Item = ActionRule>) -> Self {
        self.rules.extend(rules);
        self
    }

    pub fn with_grants(mut self, grants: impl IntoIterator<Item = UnsandboxedGrant>) -> Self {
        self.grants.extend(grants);
        self
    }

    pub fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, PolicyError> {
        self.ensure_revision(request)?;
        let digest = request.action().digest();

        if let Some(decision) = self.deterministic_rule_decision(request) {
            return Ok(decision);
        }
        if let Some(grant) = self.grants.iter().find(|grant| {
            grant.matches(
                digest,
                request.action().required_capabilities(),
                request.policy_revision(),
            )
        }) {
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

    fn deterministic_rule_decision(
        &self,
        request: &ActionReviewRequest,
    ) -> Option<ExecutionDecision> {
        if let Some(rule) = self.rules.iter().find(|rule| {
            rule.action_digest() == request.action().digest()
                && matches!(rule.effect(), RuleEffect::Deny { .. })
        }) {
            let RuleEffect::Deny { reason } = rule.effect() else {
                unreachable!("rule was selected by its deny effect");
            };
            return Some(ExecutionDecision::Block(BlockReason::DeterministicRule {
                rule_id: rule.id().clone(),
                reason: reason.clone(),
            }));
        }
        let rule = self.rules.iter().find(|rule| {
            rule.action_digest() == request.action().digest()
                && matches!(rule.effect(), RuleEffect::RequireSandbox)
        })?;
        match rule.effect() {
            RuleEffect::RequireSandbox => match (request.phase(), request.sandbox()) {
                (ActionReviewPhase::SandboxDenial(denial), _) => Some(ExecutionDecision::Block(
                    BlockReason::SandboxRequiredButUnavailable {
                        rule_id: rule.id().clone(),
                        reason: format!(
                            "the required sandbox denied the action: {}",
                            denial.reason()
                        ),
                    },
                )),
                (ActionReviewPhase::Initial, SandboxCompatibility::Supported(policy)) => {
                    Some(ExecutionDecision::RunSandboxed(*policy))
                }
                (
                    ActionReviewPhase::Initial,
                    SandboxCompatibility::Unsupported { reason }
                    | SandboxCompatibility::NotApplicable { reason },
                ) => Some(ExecutionDecision::Block(
                    BlockReason::SandboxRequiredButUnavailable {
                        rule_id: rule.id().clone(),
                        reason: reason.clone(),
                    },
                )),
            },
            RuleEffect::Deny { .. } => unreachable!("deny rules were handled first"),
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
