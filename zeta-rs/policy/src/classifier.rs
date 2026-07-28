use crate::{ActionDigest, ActionReviewRequest, CapabilitySet, PolicyRevision};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use zeta_async_utils::CancellationToken;

/// Produces advisory assessments consumed by the deterministic policy engine.
///
/// Implementations must not execute actions or grant capabilities. They must propagate
/// cancellation, bind successful assessments to the supplied request identities, and return an
/// error without a recommendation when their result cannot be trusted.
pub trait ActionClassifier: Send + Sync {
    type Error: Error + Send + Sync + 'static;

    fn classify(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ClassifierAssessment, Self::Error>;
}

/// Host-visible identity of one exact classifier assessment.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssessmentId(String);

impl AssessmentId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Derives an assessment identity from its immutable request binding and exact model output.
    pub fn from_response(
        action_digest: &ActionDigest,
        policy_revision: &PolicyRevision,
        prompt_revision: &str,
        response: impl AsRef<[u8]>,
    ) -> Self {
        let mut digest = Sha256::new();
        digest.update(action_digest.as_str().as_bytes());
        digest.update([0]);
        digest.update(policy_revision.as_str().as_bytes());
        digest.update([0]);
        digest.update(prompt_revision.as_bytes());
        digest.update([0]);
        digest.update(response.as_ref());
        let digest = digest.finalize();
        Self(format!("{digest:x}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Consequence level assigned to an action by the advisory reviewer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// How directly the current user request authorizes the proposed action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserAuthorization {
    Explicit,
    Implicit,
    Absent,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "recommendation", rename_all = "snake_case")]
pub enum ClassifierRecommendation {
    Approve {
        capabilities: CapabilitySet,
        risk: RiskLevel,
        user_authorization: UserAuthorization,
        reason: String,
    },
    ReviseAction {
        maximum_capabilities: CapabilitySet,
        reason: String,
    },
    AskUser {
        reason: String,
    },
    Deny {
        reason: String,
    },
}

/// Advisory classifier output bound by the host to the reviewed action and policy revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassifierAssessment {
    assessment_id: AssessmentId,
    action_digest: ActionDigest,
    policy_revision: PolicyRevision,
    prompt_revision: String,
    recommendation: ClassifierRecommendation,
}

impl ClassifierAssessment {
    /// Creates an advisory assessment while binding it to host-owned identities.
    ///
    /// Classifier implementations must copy these identities from the request they reviewed.
    pub fn new(
        assessment_id: AssessmentId,
        action_digest: ActionDigest,
        policy_revision: PolicyRevision,
        prompt_revision: impl Into<String>,
        recommendation: ClassifierRecommendation,
    ) -> Self {
        Self {
            assessment_id,
            action_digest,
            policy_revision,
            prompt_revision: prompt_revision.into(),
            recommendation,
        }
    }

    pub fn assessment_id(&self) -> &AssessmentId {
        &self.assessment_id
    }

    pub fn action_digest(&self) -> &ActionDigest {
        &self.action_digest
    }

    pub fn policy_revision(&self) -> &PolicyRevision {
        &self.policy_revision
    }

    pub fn prompt_revision(&self) -> &str {
        &self.prompt_revision
    }

    pub fn recommendation(&self) -> &ClassifierRecommendation {
        &self.recommendation
    }
}
