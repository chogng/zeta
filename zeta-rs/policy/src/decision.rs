use crate::{ActionDigest, AssessmentId, CapabilitySet, GrantId, PolicyRevision, RuleId};
use std::fmt;
use zeta_sandboxing::SandboxPolicy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewFailurePolicy {
    Block,
    AskUser,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequest {
    action_digest: ActionDigest,
    capabilities: CapabilitySet,
    reason: String,
}

impl ApprovalRequest {
    pub fn new(
        action_digest: ActionDigest,
        capabilities: CapabilitySet,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            action_digest,
            capabilities,
            reason: reason.into(),
        }
    }

    pub fn action_digest(&self) -> &ActionDigest {
        &self.action_digest
    }

    pub fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// One-use policy authority created from a bound reviewer assessment.
///
/// The reviewer can recommend approval, but only the policy engine can create this grant after
/// checking risk, user authorization, exact capabilities, and request identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoReviewGrant {
    assessment_id: AssessmentId,
    action_digest: ActionDigest,
    capabilities: CapabilitySet,
    policy_revision: PolicyRevision,
}

/// One-use authority derived from a Turn's explicit permission-bypass ceiling.
///
/// Hosts may create this grant only after their authoritative policy has evaluated the exact
/// action and returned an interactive approval request. Built-in denial, policy revision, action
/// digest, and complete capability binding therefore remain mandatory in bypass mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionBypassGrant {
    action_digest: ActionDigest,
    capabilities: CapabilitySet,
    policy_revision: PolicyRevision,
}

impl PermissionBypassGrant {
    pub fn new(
        action_digest: ActionDigest,
        capabilities: CapabilitySet,
        policy_revision: PolicyRevision,
    ) -> Self {
        Self {
            action_digest,
            capabilities,
            policy_revision,
        }
    }

    pub fn matches(
        &self,
        action_digest: &ActionDigest,
        capabilities: &CapabilitySet,
        policy_revision: &PolicyRevision,
    ) -> bool {
        self.action_digest == *action_digest
            && self.capabilities == *capabilities
            && self.policy_revision == *policy_revision
    }
}

impl AutoReviewGrant {
    pub(crate) fn new(
        assessment_id: AssessmentId,
        action_digest: ActionDigest,
        capabilities: CapabilitySet,
        policy_revision: PolicyRevision,
    ) -> Self {
        Self {
            assessment_id,
            action_digest,
            capabilities,
            policy_revision,
        }
    }

    pub fn assessment_id(&self) -> &AssessmentId {
        &self.assessment_id
    }

    pub fn matches(
        &self,
        action_digest: &ActionDigest,
        capabilities: &CapabilitySet,
        policy_revision: &PolicyRevision,
    ) -> bool {
        self.action_digest == *action_digest
            && self.capabilities == *capabilities
            && self.policy_revision == *policy_revision
    }
}

/// Structured constraints returned to the parent Agent for a materially safer retry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaferActionRequest {
    assessment_id: AssessmentId,
    maximum_capabilities: CapabilitySet,
    reason: String,
}

impl SaferActionRequest {
    pub(crate) fn new(
        assessment_id: AssessmentId,
        maximum_capabilities: CapabilitySet,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            assessment_id,
            maximum_capabilities,
            reason: reason.into(),
        }
    }

    pub fn assessment_id(&self) -> &AssessmentId {
        &self.assessment_id
    }

    pub fn maximum_capabilities(&self) -> &CapabilitySet {
        &self.maximum_capabilities
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlockReason {
    DeterministicRule {
        rule_id: RuleId,
        reason: String,
    },
    ReviewerDenied {
        assessment_id: AssessmentId,
        reason: String,
    },
    CriticalRisk {
        assessment_id: AssessmentId,
        reason: String,
    },
    ReviewFailed {
        reason: String,
    },
    SandboxRequiredButUnavailable {
        rule_id: RuleId,
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionDecision {
    RunSandboxed(SandboxPolicy),
    RunUnsandboxed { grant_id: GrantId },
    RunAutoReviewed(AutoReviewGrant),
    RunWithPermissionBypass(PermissionBypassGrant),
    ReviseAction(SaferActionRequest),
    AskUser(ApprovalRequest),
    Block(BlockReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    RevisionMismatch {
        engine: PolicyRevision,
        request: PolicyRevision,
    },
    ClassifierBindingMismatch,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RevisionMismatch { engine, request } => write!(
                formatter,
                "policy revision mismatch: engine={}, request={}",
                engine.as_str(),
                request.as_str()
            ),
            Self::ClassifierBindingMismatch => {
                formatter.write_str("classifier assessment was bound to another action or policy")
            }
        }
    }
}

impl std::error::Error for PolicyError {}
