use crate::GitRootTarget;
use crate::VerificationRootState;
use crate::WorkCoordinationError;
use crate::WorkVerification;
use serde::Deserialize;
use serde::Serialize;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkRunId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum IntegrationRootTarget {
    Git {
        repository_id: String,
        relative_path: String,
        target: GitRootTarget,
        target_tree: String,
        final_tree: String,
    },
    Directory {
        target_snapshot_id: String,
        final_snapshot_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum IntegrationPreparedArtifact {
    GitCommit { object_id: String },
    DirectorySnapshot { snapshot_id: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationRootStatus {
    Pending,
    Prepared,
    Published,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkIntegrationRoot {
    pub root_id: ContentDigest,
    pub source_dir_id: DirId,
    pub target: IntegrationRootTarget,
    pub status: IntegrationRootStatus,
    pub prepared_artifact: Option<IntegrationPreparedArtifact>,
    pub publication_receipt_digest: Option<ContentDigest>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkIntegrationStatus {
    Queued,
    Integrating,
    Integrated,
    Partial,
    Conflict,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationFailureKind {
    Conflict,
    Failure,
    TargetMoved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationIncident {
    pub generation: u64,
    pub kind: IntegrationFailureKind,
    pub reason: String,
    pub published_root_count: u64,
}

/// Durable publication transaction for one exact verified result set.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkIntegration {
    pub integration_key: ContentDigest,
    pub verification_key: ContentDigest,
    pub generation: u64,
    pub status: WorkIntegrationStatus,
    pub roots: Vec<WorkIntegrationRoot>,
    pub incidents: Vec<IntegrationIncident>,
    pub evidence_digest: Option<ContentDigest>,
}

pub fn integration_key(
    work_run_id: &WorkRunId,
    verification_key: &ContentDigest,
) -> Result<ContentDigest, WorkCoordinationError> {
    let encoded = serde_json::to_vec(&(1_u32, work_run_id, verification_key))
        .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}

pub(crate) fn integration_roots(
    verification: &WorkVerification,
) -> Result<Vec<WorkIntegrationRoot>, WorkCoordinationError> {
    let mut roots = Vec::new();
    for root in &verification.input.roots {
        match &root.state {
            VerificationRootState::Git { repositories } => {
                for repository in repositories {
                    roots.push(WorkIntegrationRoot {
                        root_id: integration_root_id(
                            &verification.verification_key,
                            roots.len(),
                            &root.source_dir_id,
                            &IntegrationRootTarget::Git {
                                repository_id: repository.repository_id.clone(),
                                relative_path: repository.relative_path.clone(),
                                target: repository.target.clone(),
                                target_tree: repository.target_tree.clone(),
                                final_tree: repository.final_tree.clone(),
                            },
                        )?,
                        source_dir_id: root.source_dir_id.clone(),
                        target: IntegrationRootTarget::Git {
                            repository_id: repository.repository_id.clone(),
                            relative_path: repository.relative_path.clone(),
                            target: repository.target.clone(),
                            target_tree: repository.target_tree.clone(),
                            final_tree: repository.final_tree.clone(),
                        },
                        status: IntegrationRootStatus::Pending,
                        prepared_artifact: None,
                        publication_receipt_digest: None,
                    });
                }
            }
            VerificationRootState::Directory {
                target_snapshot_id,
                final_snapshot_id,
            } => {
                let target = IntegrationRootTarget::Directory {
                    target_snapshot_id: target_snapshot_id.clone(),
                    final_snapshot_id: final_snapshot_id.clone(),
                };
                roots.push(WorkIntegrationRoot {
                    root_id: integration_root_id(
                        &verification.verification_key,
                        roots.len(),
                        &root.source_dir_id,
                        &target,
                    )?,
                    source_dir_id: root.source_dir_id.clone(),
                    target,
                    status: IntegrationRootStatus::Pending,
                    prepared_artifact: None,
                    publication_receipt_digest: None,
                });
            }
        }
    }
    if roots.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(
            "integration requires at least one publishable root".into(),
        ));
    }
    Ok(roots)
}

pub(crate) fn validate_prepared_artifact(
    root: &WorkIntegrationRoot,
    artifact: &IntegrationPreparedArtifact,
) -> Result<(), WorkCoordinationError> {
    let valid = match (&root.target, artifact) {
        (
            IntegrationRootTarget::Git { .. },
            IntegrationPreparedArtifact::GitCommit { object_id },
        ) => !object_id.trim().is_empty(),
        (
            IntegrationRootTarget::Directory {
                final_snapshot_id, ..
            },
            IntegrationPreparedArtifact::DirectorySnapshot { snapshot_id },
        ) => snapshot_id == final_snapshot_id,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(WorkCoordinationError::InvalidInput(
            "prepared integration artifact does not match its verified root".into(),
        ))
    }
}

pub(crate) fn integration_evidence_digest(
    integration: &WorkIntegration,
) -> Result<ContentDigest, WorkCoordinationError> {
    let encoded = serde_json::to_vec(&(
        1_u32,
        &integration.integration_key,
        integration.generation,
        integration.status,
        &integration.roots,
        &integration.incidents,
    ))
    .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}

fn integration_root_id(
    verification_key: &ContentDigest,
    index: usize,
    source_dir_id: &DirId,
    target: &IntegrationRootTarget,
) -> Result<ContentDigest, WorkCoordinationError> {
    let index = u64::try_from(index)
        .map_err(|_| WorkCoordinationError::InvalidInput("too many integration roots".into()))?;
    let encoded = serde_json::to_vec(&(1_u32, verification_key, index, source_dir_id, target))
        .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}
