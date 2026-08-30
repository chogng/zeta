use super::turn_changes_runtime::TurnChangesRuntime;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use zeta_git::GitClient;
use zeta_git::GitPrepareTreeCommitResult;
use zeta_git::GitPreparedTreeCommitRequest;
use zeta_git::GitTreeCommitConflict;
use zeta_git::GitTreeCommitRecovery;
use zeta_git::GitTreeCommitResult;
use zeta_git::GitTreeId;
use zeta_protocol::ContentDigest;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::IntegrationFailureKind;
use zeta_work_coordination::IntegrationPreparedArtifact;
use zeta_work_coordination::IntegrationRootStatus;
use zeta_work_coordination::IntegrationRootTarget;
use zeta_work_coordination::WorkIntegration;
use zeta_work_coordination::WorkIntegrationRoot;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkVerification;
use zeta_work_coordination::WorkVerificationStatus;

#[derive(Debug)]
pub(super) struct WorkIntegrationFailure {
    pub(super) kind: IntegrationFailureKind,
    pub(super) reason: String,
}

impl TurnChangesRuntime {
    /// Rejects publication shapes for which the host cannot provide an atomic target adapter.
    pub(super) fn validate_work_integration(
        &self,
        run: &WorkRun,
        verification_key: &ContentDigest,
    ) -> Result<(), String> {
        let verification = run
            .verifications
            .get(verification_key)
            .ok_or_else(|| format!("WorkVerification {verification_key} does not exist"))?;
        if verification.status != WorkVerificationStatus::Verified {
            return Err("integration requires a current verified result set".into());
        }
        for root in &verification.input.roots {
            match &root.state {
                zeta_work_coordination::VerificationRootState::Directory { .. } => {
                    return Err(
                        "directory roots cannot integrate until an atomic directory publication adapter is qualified"
                            .into(),
                    );
                }
                zeta_work_coordination::VerificationRootState::Git { repositories } => {
                    for repository in repositories {
                        if matches!(repository.target, GitRootTarget::Detached { .. }) {
                            return Err(format!(
                                "repository {} has a detached target and cannot be published",
                                repository.repository_id
                            ));
                        }
                        self.integration_repository_path(
                            run,
                            verification,
                            &root.source_dir_id,
                            &repository.repository_id,
                            &repository.relative_path,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Creates one unreachable deterministic commit object. No target ref is changed here.
    pub(super) fn prepare_work_integration_root(
        &self,
        run: &WorkRun,
        integration: &WorkIntegration,
        root: &WorkIntegrationRoot,
    ) -> Result<IntegrationPreparedArtifact, WorkIntegrationFailure> {
        let (repository_root, request) = self.git_integration_request(run, integration, root)?;
        self.worktree_runtime.block_on(async {
            let git = GitClient::system();
            let repository = git
                .open_repository(&repository_root)
                .await
                .map_err(git_failure)?;
            match git
                .prepare_tree_commit(&repository, &request)
                .await
                .map_err(git_failure)?
            {
                GitPrepareTreeCommitResult::Prepared(prepared) => {
                    Ok(IntegrationPreparedArtifact::GitCommit {
                        object_id: prepared.object_id().to_string(),
                    })
                }
                GitPrepareTreeCommitResult::Conflict(conflict) => Err(integration_conflict(
                    conflict,
                    "prepare verified integration root",
                )),
            }
        })
    }

    /// Recovers or publishes exactly the next prepared root and returns its durable receipt.
    pub(super) fn publish_work_integration_root(
        &self,
        run: &WorkRun,
        integration: &WorkIntegration,
        root: &WorkIntegrationRoot,
    ) -> Result<ContentDigest, WorkIntegrationFailure> {
        if root.status != IntegrationRootStatus::Prepared {
            return Err(failure(
                IntegrationFailureKind::Failure,
                "integration root is not prepared for publication",
            ));
        }
        let expected_object_id = match root.prepared_artifact.as_ref() {
            Some(IntegrationPreparedArtifact::GitCommit { object_id }) => object_id,
            _ => {
                return Err(failure(
                    IntegrationFailureKind::Failure,
                    "prepared Git root omitted its exact commit object",
                ));
            }
        };
        let (repository_root, request) = self.git_integration_request(run, integration, root)?;
        let transaction_id = integration_transaction_id(integration, root)
            .map_err(|reason| failure(IntegrationFailureKind::Failure, &reason))?;
        let published_object_id = self.worktree_runtime.block_on(async {
            let git = GitClient::system();
            let repository = git
                .open_repository(&repository_root)
                .await
                .map_err(git_failure)?;
            match git
                .recover_tree_commit(&repository, &transaction_id)
                .await
                .map_err(git_failure)?
            {
                GitTreeCommitRecovery::Committed { object_id } => {
                    if &object_id != expected_object_id {
                        return Err(failure(
                            IntegrationFailureKind::Conflict,
                            "recovered publication journal names a different commit object",
                        ));
                    }
                    return Ok(object_id);
                }
                GitTreeCommitRecovery::Conflict { paths } => {
                    return Err(failure(
                        IntegrationFailureKind::Conflict,
                        &format!("publication recovery conflicts with checkout paths {paths:?}"),
                    ));
                }
                GitTreeCommitRecovery::None | GitTreeCommitRecovery::RolledBack => {}
            }

            let prepared = match git
                .prepare_tree_commit(&repository, &request)
                .await
                .map_err(git_failure)?
            {
                GitPrepareTreeCommitResult::Prepared(prepared) => prepared,
                GitPrepareTreeCommitResult::Conflict(conflict) => {
                    return Err(integration_conflict(
                        conflict,
                        "recheck integration target before publication",
                    ));
                }
            };
            if prepared.object_id() != expected_object_id {
                return Err(failure(
                    IntegrationFailureKind::Failure,
                    "deterministic prepared commit does not match the durable artifact",
                ));
            }
            match git
                .publish_prepared_tree_commit(&repository, &prepared)
                .await
                .map_err(git_failure)?
            {
                GitTreeCommitResult::Committed { object_id } => Ok(object_id),
                GitTreeCommitResult::Conflict(conflict) => Err(integration_conflict(
                    conflict,
                    "publish verified integration root",
                )),
            }
        })?;
        publication_receipt_digest(integration, root, &published_object_id)
            .map_err(|reason| failure(IntegrationFailureKind::Failure, &reason))
    }

    /// Cleans retained Git journals only after their root publication receipts are durable.
    pub(super) fn acknowledge_work_integration_receipts(
        &self,
        run: &WorkRun,
    ) -> Result<(), String> {
        for integration in run.integrations.values() {
            let verification = run
                .verifications
                .get(&integration.verification_key)
                .ok_or_else(|| "integration verification disappeared".to_string())?;
            for root in integration
                .roots
                .iter()
                .filter(|root| root.status == IntegrationRootStatus::Published)
            {
                let IntegrationRootTarget::Git {
                    repository_id,
                    relative_path,
                    ..
                } = &root.target
                else {
                    continue;
                };
                let Some(IntegrationPreparedArtifact::GitCommit { object_id }) =
                    root.prepared_artifact.as_ref()
                else {
                    return Err("published Git root omitted its commit object".into());
                };
                let repository_root = self.integration_repository_path(
                    run,
                    verification,
                    &root.source_dir_id,
                    repository_id,
                    relative_path,
                )?;
                let transaction_id = integration_transaction_id(integration, root)?;
                self.worktree_runtime.block_on(async {
                    let git = GitClient::system();
                    let repository = git
                        .open_repository(&repository_root)
                        .await
                        .map_err(|error| error.to_string())?;
                    git.acknowledge_published_tree_commit(&repository, &transaction_id, object_id)
                        .await
                        .map_err(|error| error.to_string())
                })?;
            }
        }
        Ok(())
    }

    fn git_integration_request(
        &self,
        run: &WorkRun,
        integration: &WorkIntegration,
        root: &WorkIntegrationRoot,
    ) -> Result<(PathBuf, GitPreparedTreeCommitRequest), WorkIntegrationFailure> {
        let verification = run
            .verifications
            .get(&integration.verification_key)
            .ok_or_else(|| {
                failure(
                    IntegrationFailureKind::Failure,
                    "integration verification disappeared",
                )
            })?;
        let IntegrationRootTarget::Git {
            repository_id,
            relative_path,
            target,
            target_tree,
            final_tree,
        } = &root.target
        else {
            return Err(failure(
                IntegrationFailureKind::Failure,
                "directory publication adapter is unavailable",
            ));
        };
        let repository_root = self
            .integration_repository_path(
                run,
                verification,
                &root.source_dir_id,
                repository_id,
                relative_path,
            )
            .map_err(|reason| failure(IntegrationFailureKind::Failure, &reason))?;
        let target_tree = GitTreeId::new(target_tree.clone()).map_err(|error| {
            failure(
                IntegrationFailureKind::Failure,
                &format!("invalid verified target tree: {error}"),
            )
        })?;
        let final_tree = GitTreeId::new(final_tree.clone()).map_err(|error| {
            failure(
                IntegrationFailureKind::Failure,
                &format!("invalid verified final tree: {error}"),
            )
        })?;
        let transaction_id = integration_transaction_id(integration, root)
            .map_err(|reason| failure(IntegrationFailureKind::Failure, &reason))?;
        let message = format!(
            "Integrate verified WorkRun result\n\nWorkRun: {}\nVerification: {}\nRoot: {}",
            run.work_run_id, integration.verification_key, root.root_id
        );
        let request = match target {
            GitRootTarget::Branch {
                name,
                expected_head,
            } => GitPreparedTreeCommitRequest::new(
                transaction_id,
                name.clone(),
                expected_head.clone(),
                target_tree,
                final_tree,
                message,
            ),
            GitRootTarget::UnbornBranch { name, .. } => GitPreparedTreeCommitRequest::new_unborn(
                transaction_id,
                name.clone(),
                target_tree,
                final_tree,
                message,
            ),
            GitRootTarget::Detached { .. } => {
                return Err(failure(
                    IntegrationFailureKind::Failure,
                    "detached Git targets cannot be published",
                ));
            }
        }
        .map_err(|error| {
            failure(
                IntegrationFailureKind::Failure,
                &format!("invalid Git integration request: {error}"),
            )
        })?;
        Ok((repository_root, request))
    }

    fn integration_repository_path(
        &self,
        run: &WorkRun,
        verification: &WorkVerification,
        source_dir_id: &zeta_file_access::DirId,
        repository_id: &str,
        relative_path: &str,
    ) -> Result<PathBuf, String> {
        let mut candidates = BTreeSet::new();
        for result in &verification.input.ordered_results {
            let attempt = run
                .attempts
                .get(&result.attempt_id)
                .ok_or_else(|| "integration WorkAttempt disappeared".to_string())?;
            self.ensure_work_attempt_workspace(&run.work_run_id, attempt)?;
            let bindings = self
                .work_attempt_bindings
                .read()
                .map_err(|_| "WorkAttempt workspace binding lock poisoned".to_string())?
                .get(&(run.work_run_id.clone(), attempt.attempt_id.clone()))
                .cloned()
                .ok_or_else(|| "integration WorkAttempt workspace disappeared".to_string())?;
            let Some(root) = bindings
                .roots
                .iter()
                .find(|root| &root.checkpoint.dir_id == source_dir_id)
            else {
                continue;
            };
            let repository = root
                .managed
                .repositories()
                .iter()
                .find(|repository| repository.repository_id() == repository_id)
                .ok_or_else(|| {
                    format!("integration root omitted verified repository {repository_id}")
                })?;
            if repository.relative_path() != Path::new(relative_path) {
                return Err("integration repository relative path changed".into());
            }
            candidates.insert(repository.source_repository_root().to_path_buf());
        }
        if candidates.len() != 1 {
            return Err(format!(
                "integration repository {repository_id} has no unique source binding"
            ));
        }
        candidates
            .into_iter()
            .next()
            .ok_or_else(|| "integration repository binding is unavailable".into())
    }
}

fn integration_transaction_id(
    integration: &WorkIntegration,
    root: &WorkIntegrationRoot,
) -> Result<String, String> {
    let encoded = serde_json::to_vec(&(1_u32, &integration.integration_key, &root.root_id))
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "work-integration-{}",
        ContentDigest::sha256(&encoded)
            .to_string()
            .replace(':', "-")
    ))
}

fn publication_receipt_digest(
    integration: &WorkIntegration,
    root: &WorkIntegrationRoot,
    object_id: &str,
) -> Result<ContentDigest, String> {
    let encoded = serde_json::to_vec(&(
        1_u32,
        &integration.integration_key,
        integration.generation,
        &root.root_id,
        &root.target,
        &root.prepared_artifact,
        object_id,
    ))
    .map_err(|error| error.to_string())?;
    Ok(ContentDigest::sha256(&encoded))
}

fn integration_conflict(
    conflict: GitTreeCommitConflict,
    operation: &str,
) -> WorkIntegrationFailure {
    match conflict {
        GitTreeCommitConflict::TargetMoved | GitTreeCommitConflict::TargetDeleted => failure(
            IntegrationFailureKind::TargetMoved,
            &format!("{operation}: target branch moved after verification"),
        ),
        GitTreeCommitConflict::CheckoutChanged { paths } => failure(
            IntegrationFailureKind::Conflict,
            &format!("{operation}: checked-out target conflicts at {paths:?}"),
        ),
        GitTreeCommitConflict::TargetDetached => failure(
            IntegrationFailureKind::Conflict,
            &format!("{operation}: target checkout became detached"),
        ),
        GitTreeCommitConflict::ChangeSet { paths } => failure(
            IntegrationFailureKind::Conflict,
            &format!("{operation}: verified change conflicts at {paths:?}"),
        ),
    }
}

fn git_failure(error: zeta_git::GitError) -> WorkIntegrationFailure {
    failure(
        IntegrationFailureKind::Failure,
        &format!("Git integration failed: {error}"),
    )
}

fn failure(kind: IntegrationFailureKind, reason: &str) -> WorkIntegrationFailure {
    WorkIntegrationFailure {
        kind,
        reason: reason.into(),
    }
}
