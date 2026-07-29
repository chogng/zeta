use crate::{
    ActionReviewPhase, ActionReviewRequest, ActionRule, BlockReason, ExecutionDecision, RuleEffect,
    UnsandboxedGrant,
};

/// Host-owned deterministic safety rules evaluated before user authorization or model review.
///
/// Callers use this layer for invariants such as unconditional denial and mandatory sandboxing.
/// Implementations must bind rules to canonical action digests rather than tool names or summaries.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuiltInSafetyPolicy {
    rules: Vec<ActionRule>,
}

impl BuiltInSafetyPolicy {
    pub fn new(rules: impl IntoIterator<Item = ActionRule>) -> Self {
        Self {
            rules: rules.into_iter().collect(),
        }
    }

    pub fn extend(&mut self, rules: impl IntoIterator<Item = ActionRule>) {
        self.rules.extend(rules);
    }

    pub(crate) fn decision(&self, request: &ActionReviewRequest) -> Option<ExecutionDecision> {
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
                (ActionReviewPhase::Initial, crate::SandboxCompatibility::Supported(policy)) => {
                    Some(ExecutionDecision::RunSandboxed(*policy))
                }
                (
                    ActionReviewPhase::Initial,
                    crate::SandboxCompatibility::Unsupported { reason }
                    | crate::SandboxCompatibility::NotApplicable { reason },
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
}

/// User-authorized exact actions that may run without platform sandbox enforcement.
///
/// Entries remain bound to an action digest, complete capability set, and policy revision.
/// Adding a tool name or command prefix is intentionally insufficient to authorize execution.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserAllowlist {
    grants: Vec<UnsandboxedGrant>,
}

impl UserAllowlist {
    pub fn new(grants: impl IntoIterator<Item = UnsandboxedGrant>) -> Self {
        Self {
            grants: grants.into_iter().collect(),
        }
    }

    pub fn extend(&mut self, grants: impl IntoIterator<Item = UnsandboxedGrant>) {
        self.grants.extend(grants);
    }

    pub(crate) fn matching_grant(
        &self,
        request: &ActionReviewRequest,
    ) -> Option<&UnsandboxedGrant> {
        self.grants.iter().find(|grant| {
            grant.matches(
                request.action().digest(),
                request.action().required_capabilities(),
                request.policy_revision(),
            )
        })
    }
}
