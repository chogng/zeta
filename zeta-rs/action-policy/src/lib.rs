//! Action permission contracts and final execution-decision authority.
//!
//! This crate consumes deterministic [`zeta_execpolicy`] evaluation, exact grants, sandbox
//! compatibility, and output from an injected advisory classifier. It never executes actions;
//! callers must durably record approval interactions before honoring `AskUser`.

mod action;
mod classifier;
mod context;
mod decision;
mod engine;
mod grant;
mod grants;

pub use action::{
    ActionDigest, ActionKind, ActionPolicyRevision, ActionProvenance, ActionReviewPhase,
    ActionReviewRequest, ActionSource, Capability, CapabilityKind, CapabilitySet,
    ProcessInvocationKind, ResolvedAction, SandboxCompatibility, SandboxDenialEvidence,
};
pub use classifier::{
    ActionClassifier, AssessmentId, ClassifierAssessment, ClassifierRecommendation,
    RecommendationValidationError, RiskLevel, UserAuthorization,
};
pub use context::{ReviewContext, ReviewEvidence, ReviewEvidenceKind, ReviewEvidenceTrust};
pub use decision::{
    ApprovalRequest, AutoReviewGrant, BlockReason, DeterministicPolicyGrant, ExecutionDecision,
    PermissionBypassGrant, PolicyError, ReviewFailurePolicy, SaferActionRequest,
};
pub use engine::ActionPolicyEngine;
pub use grant::{GrantId, UnsandboxedGrant};
pub use grants::UserAllowlist;
