//! Deterministic coordination state for one or more Agent development attempts.

mod attempt;
mod attempt_reducer;
mod command;
mod contract;
mod contract_reducer;
mod coordinator;
mod dependency_graph;
mod error;
mod integration;
mod integration_reducer;
mod reducer;
mod relation;
mod relation_reducer;
mod result;
mod run;
mod snapshot_validation;
mod store;
mod validation;
mod verification;
mod verification_reducer;
mod wait_reconciliation;
mod workspace;

pub use attempt::ExternalEffectsStatus;
pub use attempt::WorkAttempt;
pub use attempt::WorkAttemptCoordinationStatus;
pub use attempt::WorkAttemptExecutionStatus;
pub use attempt::WorkAttemptIntegrationStatus;
pub use attempt::WorkAttemptResult;
pub use attempt::WorkAttemptVerificationStatus;
pub use attempt::WorkStartMode;
pub use command::ResolveWaitOutcome;
pub use command::WorkContractDraft;
pub use command::WorkRunCommand;
pub use command::WorkRunCommandRequest;
pub use contract::AuthorizationSnapshotRef;
pub use contract::ControlResourceBinding;
pub use contract::ControlResourceKind;
pub use contract::GitRepositoryCheckpoint;
pub use contract::GitRootTarget;
pub use contract::RootCheckpoint;
pub use contract::RootState;
pub use contract::ValidationProfileRef;
pub use contract::WorkContractRef;
pub use contract::WorkContractVersion;
pub use contract::WorkResultRef;
pub use contract::WorkScopeClaim;
pub use coordinator::WorkCommandDisposition;
pub use coordinator::WorkCommandResult;
pub use coordinator::WorkCoordinator;
pub use dependency_graph::ordered_result_refs;
pub use dependency_graph::ordered_result_refs_with_dependencies;
pub use error::WorkCoordinationError;
pub use integration::IntegrationFailureKind;
pub use integration::IntegrationIncident;
pub use integration::IntegrationPreparedArtifact;
pub use integration::IntegrationRootStatus;
pub use integration::IntegrationRootTarget;
pub use integration::WorkIntegration;
pub use integration::WorkIntegrationRoot;
pub use integration::WorkIntegrationStatus;
pub use integration::integration_key;
pub use relation::WorkConflict;
pub use relation::WorkConflictStatus;
pub use relation::WorkRelation;
pub use relation::WorkRelationKind;
pub use relation::WorkRelationStatus;
pub use relation::WorkWaitCondition;
pub use result::WorkAttemptChangeEvidenceRef;
pub use result::work_attempt_result_digest;
pub use run::WorkDecision;
pub use run::WorkGoal;
pub use run::WorkParticipant;
pub use run::WorkParticipantRelation;
pub use run::WorkRun;
pub use run::WorkRunStatus;
pub use store::WorkRunCommit;
pub use store::WorkRunStore;
pub use store::WorkRunStoreError;
pub use store::WorkRunStoreOutcome;
pub use validation::root_checkpoint_digest;
pub use verification::GitVerificationRepository;
pub use verification::VerificationChangeSetInput;
pub use verification::VerificationCheckEvidence;
pub use verification::VerificationCheckOutcome;
pub use verification::VerificationConclusion;
pub use verification::VerificationRoot;
pub use verification::VerificationRootState;
pub use verification::WorkSerializabilityEvidence;
pub use verification::WorkSerializabilityStatus;
pub use verification::WorkVerification;
pub use verification::WorkVerificationInput;
pub use verification::WorkVerificationStatus;
pub use verification::verification_coordination_digest;
pub use verification::verification_key;
pub use wait_reconciliation::next_wait_resolution;
pub use workspace::ManagedRootBinding;
pub use workspace::WorkAttemptWorkspace;

#[cfg(test)]
#[path = "coordination_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "reference_model_tests.rs"]
mod reference_model_tests;

#[cfg(test)]
#[path = "persistence_fault_tests.rs"]
mod persistence_fault_tests;
