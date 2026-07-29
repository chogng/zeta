//! Action permission contracts and deterministic execution-decision authority.
//!
//! This crate combines exact rules and grants with output from an injected advisory classifier. It
//! never executes actions; callers must durably record approval interactions before honoring
//! `AskUser`.

mod action;
mod classifier;
mod context;
mod decision;
mod engine;
mod layers;
mod rule;

pub use action::{
    ActionDigest, ActionKind, ActionProvenance, ActionReviewPhase, ActionReviewRequest,
    ActionSource, Capability, CapabilityKind, CapabilitySet, PolicyRevision, ProcessInvocationKind,
    ResolvedAction, SandboxCompatibility, SandboxDenialEvidence,
};
pub use classifier::{
    ActionClassifier, AssessmentId, ClassifierAssessment, ClassifierRecommendation,
    RecommendationValidationError, RiskLevel, UserAuthorization,
};
pub use context::{ReviewContext, ReviewEvidence, ReviewEvidenceKind, ReviewEvidenceTrust};
pub use decision::{
    ApprovalRequest, AutoReviewGrant, BlockReason, ExecutionDecision, PolicyError,
    ReviewFailurePolicy, SaferActionRequest,
};
pub use engine::PolicyEngine;
pub use layers::{BuiltInSafetyPolicy, UserAllowlist};
pub use rule::{ActionRule, GrantId, RuleEffect, RuleId, UnsandboxedGrant};
