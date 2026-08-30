use zeta_app_server_protocol::protocol::turn_changes::ChangeSetId;
use zeta_app_server_protocol::protocol::work_run_model::AuthorizationSnapshotRefDto;
use zeta_app_server_protocol::protocol::work_run_model::ControlResourceBindingDto;
use zeta_app_server_protocol::protocol::work_run_model::ControlResourceKindDto;
use zeta_app_server_protocol::protocol::work_run_model::ExternalEffectsStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::GitRepositoryCheckpointDto;
use zeta_app_server_protocol::protocol::work_run_model::GitRootTargetDto;
use zeta_app_server_protocol::protocol::work_run_model::GitVerificationRepositoryDto;
use zeta_app_server_protocol::protocol::work_run_model::IntegrationFailureKindDto;
use zeta_app_server_protocol::protocol::work_run_model::IntegrationIncidentDto;
use zeta_app_server_protocol::protocol::work_run_model::IntegrationPreparedArtifactDto;
use zeta_app_server_protocol::protocol::work_run_model::IntegrationRootStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::IntegrationRootTargetDto;
use zeta_app_server_protocol::protocol::work_run_model::ManagedRootBindingDto;
use zeta_app_server_protocol::protocol::work_run_model::RootCheckpointDto;
use zeta_app_server_protocol::protocol::work_run_model::RootStateDto;
use zeta_app_server_protocol::protocol::work_run_model::ValidationProfileRefDto;
use zeta_app_server_protocol::protocol::work_run_model::VerificationChangeSetInputDto;
use zeta_app_server_protocol::protocol::work_run_model::VerificationCheckEvidenceDto;
use zeta_app_server_protocol::protocol::work_run_model::VerificationCheckOutcomeDto;
use zeta_app_server_protocol::protocol::work_run_model::VerificationRootDto;
use zeta_app_server_protocol::protocol::work_run_model::VerificationRootStateDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptChangeEvidenceRefDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptCoordinationStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptExecutionStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptIntegrationStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptResultDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptVerificationStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkAttemptWorkspaceDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkConflictDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkConflictStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkContractRefDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkContractVersionDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkDecisionDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkGoalDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkIntegrationDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkIntegrationRootDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkIntegrationStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkParticipantDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkParticipantRelationDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkRelationDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkRelationKindDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkRelationStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkResultRefDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkRunDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkRunStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkScopeClaimDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkSerializabilityEvidenceDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkSerializabilityStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkVerificationDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkVerificationInputDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkVerificationStatusDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkWaitConditionDto;
use zeta_app_server_protocol::protocol::work_runs::WorkRunSummaryDto;
use zeta_work_coordination::AuthorizationSnapshotRef;
use zeta_work_coordination::ControlResourceBinding;
use zeta_work_coordination::ControlResourceKind;
use zeta_work_coordination::GitRepositoryCheckpoint;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::IntegrationFailureKind;
use zeta_work_coordination::IntegrationPreparedArtifact;
use zeta_work_coordination::IntegrationRootStatus;
use zeta_work_coordination::IntegrationRootTarget;
use zeta_work_coordination::ManagedRootBinding;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::ValidationProfileRef;
use zeta_work_coordination::VerificationCheckEvidence;
use zeta_work_coordination::VerificationCheckOutcome;
use zeta_work_coordination::VerificationRoot;
use zeta_work_coordination::VerificationRootState;
use zeta_work_coordination::WorkAttempt;
use zeta_work_coordination::WorkAttemptCoordinationStatus;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkAttemptIntegrationStatus;
use zeta_work_coordination::WorkAttemptVerificationStatus;
use zeta_work_coordination::WorkAttemptWorkspace;
use zeta_work_coordination::WorkConflict;
use zeta_work_coordination::WorkConflictStatus;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkContractVersion;
use zeta_work_coordination::WorkIntegration;
use zeta_work_coordination::WorkIntegrationRoot;
use zeta_work_coordination::WorkIntegrationStatus;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRelation;
use zeta_work_coordination::WorkRelationKind;
use zeta_work_coordination::WorkRelationStatus;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunStatus;
use zeta_work_coordination::WorkSerializabilityStatus;
use zeta_work_coordination::WorkVerification;
use zeta_work_coordination::WorkVerificationInput;
use zeta_work_coordination::WorkVerificationStatus;
use zeta_work_coordination::WorkWaitCondition;

pub(super) fn summary(run: &WorkRun) -> Result<WorkRunSummaryDto, String> {
    run.validate().map_err(|error| error.to_string())?;
    let current_goal = run
        .current_goal()
        .ok_or_else(|| "WorkRun has no current goal".to_string())?;
    Ok(WorkRunSummaryDto {
        work_run_id: run.work_run_id.clone(),
        revision: run.revision,
        topology_revision: run.topology_revision,
        status: run_status(run.status),
        objective: current_goal.objective.clone(),
        session_count: count(run.session_count())?,
        participant_count: count(run.participants.len())?,
        attempt_count: count(run.attempts.len())?,
        open_conflict_count: count(
            run.conflicts
                .values()
                .filter(|conflict| conflict.status == WorkConflictStatus::Open)
                .count(),
        )?,
    })
}

pub(super) fn work_run(run: &WorkRun) -> Result<WorkRunDto, String> {
    run.validate().map_err(|error| error.to_string())?;
    Ok(WorkRunDto {
        work_run_id: run.work_run_id.clone(),
        revision: run.revision,
        topology_revision: run.topology_revision,
        status: run_status(run.status),
        terminal_reason: run.terminal_reason.clone(),
        goals: run
            .goals
            .iter()
            .map(|goal| WorkGoalDto {
                revision: goal.revision,
                objective: goal.objective.clone(),
                acceptance_conditions: goal.acceptance_conditions.clone(),
                exclusions: goal.exclusions.clone(),
            })
            .collect(),
        participants: run.participants.values().map(participant).collect(),
        decisions: run
            .decisions
            .values()
            .map(|decision| WorkDecisionDto {
                decision_id: decision.decision_id.clone(),
                authority: decision.authority.clone(),
                scope: decision.scope.clone(),
                statement: decision.statement.clone(),
                content_digest: decision.content_digest.clone(),
            })
            .collect(),
        contracts: run.contracts.values().flatten().map(contract).collect(),
        attempts: run.attempts.values().map(attempt).collect(),
        relations: run.relations.values().map(relation).collect(),
        conflicts: run.conflicts.values().map(conflict).collect(),
        verifications: run.verifications.values().map(verification).collect(),
        integrations: run.integrations.values().map(integration).collect(),
    })
}

fn integration(value: &WorkIntegration) -> WorkIntegrationDto {
    WorkIntegrationDto {
        integration_key: value.integration_key.clone(),
        verification_key: value.verification_key.clone(),
        generation: value.generation,
        status: match value.status {
            WorkIntegrationStatus::Queued => WorkIntegrationStatusDto::Queued,
            WorkIntegrationStatus::Integrating => WorkIntegrationStatusDto::Integrating,
            WorkIntegrationStatus::Integrated => WorkIntegrationStatusDto::Integrated,
            WorkIntegrationStatus::Partial => WorkIntegrationStatusDto::Partial,
            WorkIntegrationStatus::Conflict => WorkIntegrationStatusDto::Conflict,
            WorkIntegrationStatus::Failed => WorkIntegrationStatusDto::Failed,
        },
        roots: value.roots.iter().map(integration_root).collect(),
        incidents: value
            .incidents
            .iter()
            .map(|incident| IntegrationIncidentDto {
                generation: incident.generation,
                kind: match incident.kind {
                    IntegrationFailureKind::Conflict => IntegrationFailureKindDto::Conflict,
                    IntegrationFailureKind::Failure => IntegrationFailureKindDto::Failure,
                    IntegrationFailureKind::TargetMoved => IntegrationFailureKindDto::TargetMoved,
                },
                reason: incident.reason.clone(),
                published_root_count: incident.published_root_count,
            })
            .collect(),
        evidence_digest: value.evidence_digest.clone(),
    }
}

fn integration_root(value: &WorkIntegrationRoot) -> WorkIntegrationRootDto {
    WorkIntegrationRootDto {
        root_id: value.root_id.clone(),
        source_dir_id: value.source_dir_id.clone(),
        target: match &value.target {
            IntegrationRootTarget::Git {
                repository_id,
                relative_path,
                target,
                target_tree,
                final_tree,
            } => IntegrationRootTargetDto::Git {
                repository_id: repository_id.clone(),
                relative_path: relative_path.clone(),
                target: git_target(target),
                target_tree: target_tree.clone(),
                final_tree: final_tree.clone(),
            },
            IntegrationRootTarget::Directory {
                target_snapshot_id,
                final_snapshot_id,
            } => IntegrationRootTargetDto::Directory {
                target_snapshot_id: target_snapshot_id.clone(),
                final_snapshot_id: final_snapshot_id.clone(),
            },
        },
        status: match value.status {
            IntegrationRootStatus::Pending => IntegrationRootStatusDto::Pending,
            IntegrationRootStatus::Prepared => IntegrationRootStatusDto::Prepared,
            IntegrationRootStatus::Published => IntegrationRootStatusDto::Published,
        },
        prepared_artifact: value
            .prepared_artifact
            .as_ref()
            .map(|artifact| match artifact {
                IntegrationPreparedArtifact::GitCommit { object_id } => {
                    IntegrationPreparedArtifactDto::GitCommit {
                        object_id: object_id.clone(),
                    }
                }
                IntegrationPreparedArtifact::DirectorySnapshot { snapshot_id } => {
                    IntegrationPreparedArtifactDto::DirectorySnapshot {
                        snapshot_id: snapshot_id.clone(),
                    }
                }
            }),
        publication_receipt_digest: value.publication_receipt_digest.clone(),
    }
}

fn verification(value: &WorkVerification) -> WorkVerificationDto {
    WorkVerificationDto {
        verification_key: value.verification_key.clone(),
        input: verification_input(&value.input),
        status: match value.status {
            WorkVerificationStatus::Verifying => WorkVerificationStatusDto::Verifying,
            WorkVerificationStatus::Verified => WorkVerificationStatusDto::Verified,
            WorkVerificationStatus::Rejected => WorkVerificationStatusDto::Rejected,
            WorkVerificationStatus::Indeterminate => WorkVerificationStatusDto::Indeterminate,
            WorkVerificationStatus::Stale => WorkVerificationStatusDto::Stale,
        },
        checks: value.checks.iter().map(verification_check).collect(),
        evidence_digest: value.evidence_digest.clone(),
        reason: value.reason.clone(),
        stale_reason: value.stale_reason.clone(),
    }
}

fn verification_input(value: &WorkVerificationInput) -> WorkVerificationInputDto {
    WorkVerificationInputDto {
        goal_revision: value.goal_revision,
        topology_revision: value.topology_revision,
        coordination_digest: value.coordination_digest.clone(),
        ordered_results: value
            .ordered_results
            .iter()
            .map(|result| WorkResultRefDto {
                attempt_id: result.attempt_id.clone(),
                result_digest: result.result_digest.clone(),
            })
            .collect(),
        ordered_change_sets: value
            .ordered_change_sets
            .iter()
            .map(|change| VerificationChangeSetInputDto {
                attempt_id: change.attempt_id.clone(),
                change_set: WorkAttemptChangeEvidenceRefDto {
                    change_set_id: ChangeSetId(change.change_set.change_set_id.as_str().into()),
                    evidence_digest: change.change_set.evidence_digest.clone(),
                },
            })
            .collect(),
        serializability: WorkSerializabilityEvidenceDto {
            status: match value.serializability.status {
                WorkSerializabilityStatus::Proven => WorkSerializabilityStatusDto::Proven,
                WorkSerializabilityStatus::Indeterminate => {
                    WorkSerializabilityStatusDto::Indeterminate
                }
            },
            evidence_digest: value.serializability.evidence_digest.clone(),
            reason: value.serializability.reason.clone(),
        },
        roots: value.roots.iter().map(verification_root).collect(),
        authorization_digests: value.authorization_digests.clone(),
        control_resource_digests: value.control_resource_digests.clone(),
        validation_profile_digests: value.validation_profile_digests.clone(),
        validator_digest: value.validator_digest.clone(),
        environment_digest: value.environment_digest.clone(),
    }
}

fn verification_root(value: &VerificationRoot) -> VerificationRootDto {
    VerificationRootDto {
        source_dir_id: value.source_dir_id.clone(),
        checkpoint_digest: value.checkpoint_digest.clone(),
        state: match &value.state {
            VerificationRootState::Git { repositories } => VerificationRootStateDto::Git {
                repositories: repositories
                    .iter()
                    .map(|repository| GitVerificationRepositoryDto {
                        repository_id: repository.repository_id.clone(),
                        relative_path: repository.relative_path.clone(),
                        target: git_target(&repository.target),
                        target_tree: repository.target_tree.clone(),
                        final_tree: repository.final_tree.clone(),
                    })
                    .collect(),
            },
            VerificationRootState::Directory {
                target_snapshot_id,
                final_snapshot_id,
            } => VerificationRootStateDto::Directory {
                target_snapshot_id: target_snapshot_id.clone(),
                final_snapshot_id: final_snapshot_id.clone(),
            },
        },
    }
}

fn verification_check(value: &VerificationCheckEvidence) -> VerificationCheckEvidenceDto {
    VerificationCheckEvidenceDto {
        check_id: value.check_id.clone(),
        command_digest: value.command_digest.clone(),
        output_digest: value.output_digest.clone(),
        outcome: match value.outcome {
            VerificationCheckOutcome::Passed => VerificationCheckOutcomeDto::Passed,
            VerificationCheckOutcome::Failed => VerificationCheckOutcomeDto::Failed,
            VerificationCheckOutcome::Indeterminate => VerificationCheckOutcomeDto::Indeterminate,
        },
    }
}

fn count(value: usize) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| "WorkRun collection is too large".into())
}

fn run_status(status: WorkRunStatus) -> WorkRunStatusDto {
    match status {
        WorkRunStatus::Active => WorkRunStatusDto::Active,
        WorkRunStatus::Completed => WorkRunStatusDto::Completed,
        WorkRunStatus::Cancelled => WorkRunStatusDto::Cancelled,
    }
}

fn participant(value: &WorkParticipant) -> WorkParticipantDto {
    WorkParticipantDto {
        session_id: value.session_id.clone(),
        thread_id: value.thread_id.clone(),
        relation: match &value.relation {
            WorkParticipantRelation::Root => WorkParticipantRelationDto::Root,
            WorkParticipantRelation::Delegated {
                parent_thread_id,
                delegation_id,
            } => WorkParticipantRelationDto::Delegated {
                parent_thread_id: parent_thread_id.clone(),
                delegation_id: delegation_id.clone(),
            },
        },
    }
}

fn contract(value: &WorkContractVersion) -> WorkContractVersionDto {
    WorkContractVersionDto {
        contract_id: value.contract_id.clone(),
        revision: value.revision,
        goal_revision: value.goal_revision,
        topology_revision: value.topology_revision,
        owner_thread_id: value.owner_thread_id.clone(),
        objective: value.objective.clone(),
        acceptance_conditions: value.acceptance_conditions.clone(),
        exclusions: value.exclusions.clone(),
        environment_id: value.environment_id.clone(),
        roots: value.roots.iter().map(root).collect(),
        primary_root_dir_id: value.primary_root_dir_id.clone(),
        authorization: authorization(&value.authorization),
        decision_ids: value.decision_ids.clone(),
        upstream_results: value
            .upstream_results
            .iter()
            .map(|result| WorkResultRefDto {
                attempt_id: result.attempt_id.clone(),
                result_digest: result.result_digest.clone(),
            })
            .collect(),
        expected_scope: WorkScopeClaimDto {
            components: value.expected_scope.components.clone(),
            paths: value.expected_scope.paths.clone(),
            contracts: value.expected_scope.contracts.clone(),
            resources: value.expected_scope.resources.clone(),
        },
        validation_profile: validation_profile(&value.validation_profile),
    }
}

fn workspace(value: &WorkAttemptWorkspace) -> WorkAttemptWorkspaceDto {
    match value {
        WorkAttemptWorkspace::Provisioning => WorkAttemptWorkspaceDto::Provisioning,
        WorkAttemptWorkspace::Ready {
            roots,
            private_output_dir_id,
        } => WorkAttemptWorkspaceDto::Ready {
            roots: roots.iter().map(managed_root).collect(),
            private_output_dir_id: private_output_dir_id.clone(),
        },
        WorkAttemptWorkspace::Failed { reason } => WorkAttemptWorkspaceDto::Failed {
            reason: reason.clone(),
        },
    }
}

fn managed_root(value: &ManagedRootBinding) -> ManagedRootBindingDto {
    ManagedRootBindingDto {
        source_dir_id: value.source_dir_id.clone(),
        managed_dir_id: value.managed_dir_id.clone(),
        root_checkpoint_digest: value.root_checkpoint_digest.clone(),
        binding_manifest_digest: value.binding_manifest_digest.clone(),
    }
}

fn authorization(value: &AuthorizationSnapshotRef) -> AuthorizationSnapshotRefDto {
    AuthorizationSnapshotRefDto {
        authority: value.authority.clone(),
        policy_revision: value.policy_revision.clone(),
        grant_set_digest: value.grant_set_digest.clone(),
        granted_effects_digest: value.granted_effects_digest.clone(),
    }
}

fn validation_profile(value: &ValidationProfileRef) -> ValidationProfileRefDto {
    ValidationProfileRefDto {
        name: value.name.clone(),
        content_digest: value.content_digest.clone(),
    }
}

fn root(value: &RootCheckpoint) -> RootCheckpointDto {
    RootCheckpointDto {
        environment_id: value.environment_id.clone(),
        dir_id: value.dir_id.clone(),
        state: root_state(&value.state),
        control_resources: value
            .control_resources
            .iter()
            .map(control_resource)
            .collect(),
    }
}

fn root_state(value: &RootState) -> RootStateDto {
    match value {
        RootState::Git { repositories } => RootStateDto::Git {
            repositories: repositories.iter().map(git_repository).collect(),
        },
        RootState::Directory { snapshot_id } => RootStateDto::Directory {
            snapshot_id: snapshot_id.clone(),
        },
    }
}

fn git_repository(value: &GitRepositoryCheckpoint) -> GitRepositoryCheckpointDto {
    GitRepositoryCheckpointDto {
        repository_id: value.repository_id.clone(),
        relative_path: value.relative_path.clone(),
        target: git_target(&value.target),
        baseline_tree: value.baseline_tree.clone(),
    }
}

fn git_target(value: &GitRootTarget) -> GitRootTargetDto {
    match value {
        GitRootTarget::Branch {
            name,
            expected_head,
        } => GitRootTargetDto::Branch {
            name: name.clone(),
            expected_head: expected_head.clone(),
        },
        GitRootTarget::UnbornBranch {
            name,
            anchor_object_id,
        } => GitRootTargetDto::UnbornBranch {
            name: name.clone(),
            anchor_object_id: anchor_object_id.clone(),
        },
        GitRootTarget::Detached { object_id } => GitRootTargetDto::Detached {
            object_id: object_id.clone(),
        },
    }
}

fn control_resource(value: &ControlResourceBinding) -> ControlResourceBindingDto {
    ControlResourceBindingDto {
        kind: match value.kind {
            ControlResourceKind::ProjectInstructions => ControlResourceKindDto::ProjectInstructions,
            ControlResourceKind::AgentDefinition => ControlResourceKindDto::AgentDefinition,
            ControlResourceKind::Skill => ControlResourceKindDto::Skill,
            ControlResourceKind::Hook => ControlResourceKindDto::Hook,
            ControlResourceKind::BuildEntry => ControlResourceKindDto::BuildEntry,
            ControlResourceKind::ValidationProfile => ControlResourceKindDto::ValidationProfile,
            ControlResourceKind::PermissionPolicy => ControlResourceKindDto::PermissionPolicy,
            ControlResourceKind::CoordinationPolicy => ControlResourceKindDto::CoordinationPolicy,
        },
        source_dir_id: value.source_dir_id.clone(),
        relative_path: value.relative_path.clone(),
        scope: value.scope.clone(),
        precedence: value.precedence,
        content_digest: value.content_digest.clone(),
    }
}

fn attempt(value: &WorkAttempt) -> WorkAttemptDto {
    WorkAttemptDto {
        attempt_id: value.attempt_id.clone(),
        contract: contract_ref(&value.contract),
        session_id: value.session_id.clone(),
        thread_id: value.thread_id.clone(),
        environment_id: value.environment_id.clone(),
        roots: value.roots.iter().map(root).collect(),
        primary_root_dir_id: value.primary_root_dir_id.clone(),
        workspace: workspace(&value.workspace),
        execution_id: value.execution_id.clone(),
        execution_status: execution_status(value.execution_status),
        coordination_status: coordination_status(value.coordination_status),
        verification_status: verification_status(value.verification_status),
        integration_status: integration_status(value.integration_status),
        waiting_relation_id: value.waiting_relation_id.clone(),
        scope_expansion_evidence: value.scope_expansion_evidence.clone(),
        result: value.result.as_ref().map(|result| WorkAttemptResultDto {
            result_digest: result.result_digest.clone(),
            change_set_ids: result
                .change_set_ids
                .iter()
                .map(|identity| ChangeSetId(identity.as_str().into()))
                .collect(),
            private_output_digest: result.private_output_digest.clone(),
            external_effects_digest: result.external_effects_digest.clone(),
            external_effects_status: match result.external_effects_status {
                zeta_work_coordination::ExternalEffectsStatus::None => {
                    ExternalEffectsStatusDto::None
                }
                zeta_work_coordination::ExternalEffectsStatus::Verified => {
                    ExternalEffectsStatusDto::Verified
                }
                zeta_work_coordination::ExternalEffectsStatus::Unknown => {
                    ExternalEffectsStatusDto::Unknown
                }
            },
        }),
        failure: value.failure.clone(),
    }
}

fn contract_ref(value: &WorkContractRef) -> WorkContractRefDto {
    WorkContractRefDto {
        contract_id: value.contract_id.clone(),
        revision: value.revision,
    }
}

fn execution_status(value: WorkAttemptExecutionStatus) -> WorkAttemptExecutionStatusDto {
    match value {
        WorkAttemptExecutionStatus::Planned => WorkAttemptExecutionStatusDto::Planned,
        WorkAttemptExecutionStatus::Exploring => WorkAttemptExecutionStatusDto::Exploring,
        WorkAttemptExecutionStatus::Writing => WorkAttemptExecutionStatusDto::Writing,
        WorkAttemptExecutionStatus::Waiting => WorkAttemptExecutionStatusDto::Waiting,
        WorkAttemptExecutionStatus::Sealed => WorkAttemptExecutionStatusDto::Sealed,
        WorkAttemptExecutionStatus::Failed => WorkAttemptExecutionStatusDto::Failed,
        WorkAttemptExecutionStatus::Interrupted => WorkAttemptExecutionStatusDto::Interrupted,
        WorkAttemptExecutionStatus::Cancelled => WorkAttemptExecutionStatusDto::Cancelled,
    }
}

fn coordination_status(value: WorkAttemptCoordinationStatus) -> WorkAttemptCoordinationStatusDto {
    match value {
        WorkAttemptCoordinationStatus::Clear => WorkAttemptCoordinationStatusDto::Clear,
        WorkAttemptCoordinationStatus::ExpansionRequested => {
            WorkAttemptCoordinationStatusDto::ExpansionRequested
        }
        WorkAttemptCoordinationStatus::Conflict => WorkAttemptCoordinationStatusDto::Conflict,
        WorkAttemptCoordinationStatus::Stale => WorkAttemptCoordinationStatusDto::Stale,
        WorkAttemptCoordinationStatus::Blocked => WorkAttemptCoordinationStatusDto::Blocked,
    }
}

fn verification_status(value: WorkAttemptVerificationStatus) -> WorkAttemptVerificationStatusDto {
    match value {
        WorkAttemptVerificationStatus::Pending => WorkAttemptVerificationStatusDto::Pending,
        WorkAttemptVerificationStatus::Verifying => WorkAttemptVerificationStatusDto::Verifying,
        WorkAttemptVerificationStatus::Verified => WorkAttemptVerificationStatusDto::Verified,
        WorkAttemptVerificationStatus::Rejected => WorkAttemptVerificationStatusDto::Rejected,
        WorkAttemptVerificationStatus::Indeterminate => {
            WorkAttemptVerificationStatusDto::Indeterminate
        }
        WorkAttemptVerificationStatus::Stale => WorkAttemptVerificationStatusDto::Stale,
    }
}

fn integration_status(value: WorkAttemptIntegrationStatus) -> WorkAttemptIntegrationStatusDto {
    match value {
        WorkAttemptIntegrationStatus::Idle => WorkAttemptIntegrationStatusDto::Idle,
        WorkAttemptIntegrationStatus::Queued => WorkAttemptIntegrationStatusDto::Queued,
        WorkAttemptIntegrationStatus::Integrating => WorkAttemptIntegrationStatusDto::Integrating,
        WorkAttemptIntegrationStatus::Integrated => WorkAttemptIntegrationStatusDto::Integrated,
        WorkAttemptIntegrationStatus::Partial => WorkAttemptIntegrationStatusDto::Partial,
        WorkAttemptIntegrationStatus::Conflict => WorkAttemptIntegrationStatusDto::Conflict,
        WorkAttemptIntegrationStatus::Failed => WorkAttemptIntegrationStatusDto::Failed,
    }
}

fn relation(value: &WorkRelation) -> WorkRelationDto {
    WorkRelationDto {
        relation_id: value.relation_id.clone(),
        source_attempt_id: value.source_attempt_id.clone(),
        target_attempt_id: value.target_attempt_id.clone(),
        kind: relation_kind(&value.kind),
        status: relation_status(&value.status),
        resume_execution_status: value.resume_execution_status.map(execution_status),
    }
}

fn relation_kind(value: &WorkRelationKind) -> WorkRelationKindDto {
    match value {
        WorkRelationKind::Observation => WorkRelationKindDto::Observation,
        WorkRelationKind::Wait {
            target_execution_id,
            condition,
        } => WorkRelationKindDto::Wait {
            target_execution_id: target_execution_id.clone(),
            condition: match condition {
                WorkWaitCondition::ExecutionFinished => WorkWaitConditionDto::ExecutionFinished,
                WorkWaitCondition::AttemptSealed => WorkWaitConditionDto::AttemptSealed,
                WorkWaitCondition::ExactResult { result_digest } => {
                    WorkWaitConditionDto::ExactResult {
                        result_digest: result_digest.clone(),
                    }
                }
            },
        },
        WorkRelationKind::Alternate => WorkRelationKindDto::Alternate,
        WorkRelationKind::Handoff { target_contract } => WorkRelationKindDto::Handoff {
            target_contract: contract_ref(target_contract),
        },
        WorkRelationKind::ResultDependency { result_digest } => {
            WorkRelationKindDto::ResultDependency {
                result_digest: result_digest.clone(),
            }
        }
    }
}

fn relation_status(value: &WorkRelationStatus) -> WorkRelationStatusDto {
    match value {
        WorkRelationStatus::Active => WorkRelationStatusDto::Active,
        WorkRelationStatus::Waiting => WorkRelationStatusDto::Waiting,
        WorkRelationStatus::Satisfied { evidence_digest } => WorkRelationStatusDto::Satisfied {
            evidence_digest: evidence_digest.clone(),
        },
        WorkRelationStatus::Failed { reason } => WorkRelationStatusDto::Failed {
            reason: reason.clone(),
        },
        WorkRelationStatus::Cancelled => WorkRelationStatusDto::Cancelled,
        WorkRelationStatus::Stale => WorkRelationStatusDto::Stale,
    }
}

fn conflict(value: &WorkConflict) -> WorkConflictDto {
    WorkConflictDto {
        conflict_id: value.conflict_id.clone(),
        attempt_ids: value.attempt_ids.clone(),
        resource: value.resource.clone(),
        evidence: value.evidence.clone(),
        status: match value.status {
            WorkConflictStatus::Open => WorkConflictStatusDto::Open,
            WorkConflictStatus::Resolved => WorkConflictStatusDto::Resolved,
        },
        resolution_decision_id: value.resolution_decision_id.clone(),
    }
}
