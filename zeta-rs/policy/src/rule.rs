use crate::{ActionDigest, CapabilitySet, PolicyRevision};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuleId(String);

impl RuleId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GrantId(String);

impl GrantId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleEffect {
    RequireSandbox,
    Deny { reason: String },
}

/// Exact deterministic rule bound to one canonical action digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRule {
    id: RuleId,
    action_digest: ActionDigest,
    effect: RuleEffect,
}

impl ActionRule {
    pub fn new(id: RuleId, action_digest: ActionDigest, effect: RuleEffect) -> Self {
        Self {
            id,
            action_digest,
            effect,
        }
    }

    pub fn id(&self) -> &RuleId {
        &self.id
    }

    pub fn action_digest(&self) -> &ActionDigest {
        &self.action_digest
    }

    pub fn effect(&self) -> &RuleEffect {
        &self.effect
    }
}

/// Explicit authority to execute one exact action without platform sandbox enforcement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsandboxedGrant {
    id: GrantId,
    action_digest: ActionDigest,
    capabilities: CapabilitySet,
    policy_revision: PolicyRevision,
}

impl UnsandboxedGrant {
    pub fn new(
        id: GrantId,
        action_digest: ActionDigest,
        capabilities: CapabilitySet,
        policy_revision: PolicyRevision,
    ) -> Self {
        Self {
            id,
            action_digest,
            capabilities,
            policy_revision,
        }
    }

    pub fn id(&self) -> &GrantId {
        &self.id
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
