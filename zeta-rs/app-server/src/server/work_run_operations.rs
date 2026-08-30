use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::core_error;
use super::decode;
use super::result;
use super::work_run_projection;
use serde_json::Value;
use std::collections::BTreeSet;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::work_run_model::WorkParticipantRelationDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkRelationKindDto;
use zeta_app_server_protocol::protocol::work_run_model::WorkWaitConditionDto;
use zeta_app_server_protocol::protocol::work_runs::WorkRunCancelParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunChanged;
use zeta_app_server_protocol::protocol::work_runs::WorkRunCollaborationModeDto;
use zeta_app_server_protocol::protocol::work_runs::WorkRunCommandDispositionDto;
use zeta_app_server_protocol::protocol::work_runs::WorkRunCreateParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunGoalReviseParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunIntegrationRequestParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunListParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunListResult;
use zeta_app_server_protocol::protocol::work_runs::WorkRunMutationResult;
use zeta_app_server_protocol::protocol::work_runs::WorkRunParticipantAddParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunReadParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunReadResult;
use zeta_app_server_protocol::protocol::work_runs::WorkRunRelationCreateParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunSessionTreeDto;
use zeta_app_server_protocol::protocol::work_runs::WorkRunVerificationRequestParams;
use zeta_app_server_protocol::protocol::work_runs::WorkRunViewReadResult;
use zeta_core::ThreadSnapshot;
use zeta_work_coordination::WorkCommandDisposition;
use zeta_work_coordination::WorkCommandResult;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkCoordinationError;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRelationKind;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;
use zeta_work_coordination::WorkWaitCondition;

impl AppServer {
    pub(super) fn work_run_list(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let _: WorkRunListParams = decode(params)?;
        let coordinator = self.work_coordinator(connection)?;
        let work_runs = coordinator
            .list()
            .map_err(work_coordination_error)?
            .iter()
            .map(work_run_projection::summary)
            .collect::<Result<Vec<_>, _>>()
            .map_err(work_projection_error)?;
        result(&WorkRunListResult { work_runs })
    }

    pub(super) fn work_run_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunReadParams = decode(params)?;
        let work_run = self
            .work_coordinator(connection)?
            .read(&params.work_run_id)
            .map_err(work_coordination_error)?;
        result(&WorkRunReadResult {
            work_run: work_run_projection::work_run(&work_run).map_err(work_projection_error)?,
        })
    }

    pub(super) fn work_run_view_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunReadParams = decode(params)?;
        let work_run = self
            .work_coordinator(connection)?
            .read(&params.work_run_id)
            .map_err(work_coordination_error)?;
        let session_ids = work_run
            .participants
            .values()
            .map(|participant| participant.session_id.clone())
            .collect::<BTreeSet<_>>();
        let collaboration_mode = if session_ids.len() > 1 {
            WorkRunCollaborationModeDto::MultiSession
        } else if work_run.participants.values().any(|participant| {
            matches!(
                participant.relation,
                WorkParticipantRelation::Delegated { .. }
            )
        }) {
            WorkRunCollaborationModeDto::Team
        } else {
            WorkRunCollaborationModeDto::SingleAgent
        };
        let session_trees = session_ids
            .into_iter()
            .map(|session_id| {
                let threads = self
                    .threads
                    .list_session_threads(&session_id)
                    .map_err(core_error)?;
                Ok(WorkRunSessionTreeDto {
                    session_id,
                    agent_tree: zeta_core::project_agent_tree(&threads),
                })
            })
            .collect::<Result<Vec<_>, RpcError>>()?;
        result(&WorkRunViewReadResult {
            work_run: work_run_projection::work_run(&work_run).map_err(work_projection_error)?,
            collaboration_mode,
            session_trees,
        })
    }

    pub(super) fn work_run_create(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunCreateParams = decode(params)?;
        let participant = WorkParticipant {
            session_id: params.root_session_id,
            thread_id: params.root_thread_id,
            relation: WorkParticipantRelation::Root,
        };
        self.validate_work_participant(&participant)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: 0,
                command: WorkRunCommand::Create {
                    objective: params.objective,
                    acceptance_conditions: params.acceptance_conditions,
                    exclusions: params.exclusions,
                    root_participant: participant,
                },
            },
        )
    }

    pub(super) fn work_run_participant_add(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunParticipantAddParams = decode(params)?;
        let participant = WorkParticipant {
            session_id: params.session_id,
            thread_id: params.thread_id,
            relation: participant_relation(params.relation),
        };
        self.validate_work_participant(&participant)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::AddParticipant { participant },
            },
        )
    }

    pub(super) fn work_run_relation_create(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunRelationCreateParams = decode(params)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::CreateRelation {
                    relation_id: params.relation_id,
                    source_attempt_id: params.source_attempt_id,
                    target_attempt_id: params.target_attempt_id,
                    kind: relation_kind(params.kind),
                },
            },
        )
    }

    pub(super) fn work_run_goal_revise(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunGoalReviseParams = decode(params)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::ReviseGoal {
                    objective: params.objective,
                    acceptance_conditions: params.acceptance_conditions,
                    exclusions: params.exclusions,
                },
            },
        )
    }

    pub(super) fn work_run_cancel(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunCancelParams = decode(params)?;
        self.apply_work_run(
            connection,
            WorkRunCommandRequest {
                command_id: params.command_id,
                work_run_id: params.work_run_id,
                expected_revision: params.expected_revision,
                command: WorkRunCommand::Cancel {
                    reason: params.reason,
                },
            },
        )
    }

    pub(super) fn work_run_verification_request(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunVerificationRequestParams = decode(params)?;
        let selected = params.attempt_ids.iter().cloned().collect::<BTreeSet<_>>();
        if selected.is_empty() || selected.len() != params.attempt_ids.len() {
            return Err(work_coordination_error(
                WorkCoordinationError::InvalidInput(
                    "verification request requires unique WorkAttempt identities".into(),
                ),
            ));
        }
        let command = self
            .work_coordinator(connection)?
            .request_verification(
                params.command_id,
                params.work_run_id,
                params.expected_revision,
                selected,
            )
            .map_err(work_coordination_error)?;
        let response = mutation_result(&command)?;
        if command.disposition == WorkCommandDisposition::Committed {
            self.updates.publish_work_run_changed(WorkRunChanged {
                work_run: response.work_run.clone(),
            });
        }
        result(&response)
    }

    pub(super) fn work_run_integration_request(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: WorkRunIntegrationRequestParams = decode(params)?;
        let command = self
            .work_coordinator(connection)?
            .request_integration(
                params.command_id,
                params.work_run_id,
                params.expected_revision,
                params.verification_key,
            )
            .map_err(work_coordination_error)?;
        let response = mutation_result(&command)?;
        if command.disposition == WorkCommandDisposition::Committed {
            self.updates.publish_work_run_changed(WorkRunChanged {
                work_run: response.work_run.clone(),
            });
        }
        result(&response)
    }

    pub(super) fn apply_work_run(
        &self,
        connection: &ConnectionState,
        request: WorkRunCommandRequest,
    ) -> Result<Value, RpcError> {
        let command = self
            .work_coordinator(connection)?
            .apply(request)
            .map_err(work_coordination_error)?;
        let response = mutation_result(&command)?;
        if command.disposition == WorkCommandDisposition::Committed {
            self.updates.publish_work_run_changed(WorkRunChanged {
                work_run: response.work_run.clone(),
            });
        }
        result(&response)
    }

    fn work_coordinator(
        &self,
        connection: &ConnectionState,
    ) -> Result<&super::work_coordination_runtime::WorkCoordinationRuntime, RpcError> {
        require_work_coordination_host(connection)?;
        self.work_coordination
            .as_deref()
            .ok_or_else(work_coordination_unavailable)
    }

    fn validate_work_participant(&self, participant: &WorkParticipant) -> Result<(), RpcError> {
        let thread = self
            .threads
            .read_thread(&participant.thread_id)
            .map_err(|_| invalid_topology())?;
        if thread.session_id != participant.session_id {
            return Err(invalid_topology());
        }
        match &participant.relation {
            WorkParticipantRelation::Root => validate_root_thread(&thread),
            WorkParticipantRelation::Delegated {
                parent_thread_id,
                delegation_id,
            } => {
                let parent = self
                    .threads
                    .read_thread(parent_thread_id)
                    .map_err(|_| invalid_topology())?;
                let seed = thread
                    .agent_context_seed
                    .as_ref()
                    .ok_or_else(invalid_topology)?;
                if parent.session_id != participant.session_id
                    || thread.parent_thread_id.as_ref() != Some(parent_thread_id)
                    || seed.parent_thread_id != *parent_thread_id
                    || seed.delegation_id != *delegation_id
                {
                    return Err(invalid_topology());
                }
                Ok(())
            }
        }
    }
}

fn validate_root_thread(thread: &ThreadSnapshot) -> Result<(), RpcError> {
    if thread.parent_thread_id.is_some()
        || thread.forked_from_id.is_some()
        || thread.agent_context_seed.is_some()
    {
        Err(invalid_topology())
    } else {
        Ok(())
    }
}

fn participant_relation(value: WorkParticipantRelationDto) -> WorkParticipantRelation {
    match value {
        WorkParticipantRelationDto::Root => WorkParticipantRelation::Root,
        WorkParticipantRelationDto::Delegated {
            parent_thread_id,
            delegation_id,
        } => WorkParticipantRelation::Delegated {
            parent_thread_id,
            delegation_id,
        },
    }
}

fn relation_kind(value: WorkRelationKindDto) -> WorkRelationKind {
    match value {
        WorkRelationKindDto::Observation => WorkRelationKind::Observation,
        WorkRelationKindDto::Wait {
            target_execution_id,
            condition,
        } => WorkRelationKind::Wait {
            target_execution_id,
            condition: match condition {
                WorkWaitConditionDto::ExecutionFinished => WorkWaitCondition::ExecutionFinished,
                WorkWaitConditionDto::AttemptSealed => WorkWaitCondition::AttemptSealed,
                WorkWaitConditionDto::ExactResult { result_digest } => {
                    WorkWaitCondition::ExactResult { result_digest }
                }
            },
        },
        WorkRelationKindDto::Alternate => WorkRelationKind::Alternate,
        WorkRelationKindDto::Handoff { target_contract } => WorkRelationKind::Handoff {
            target_contract: WorkContractRef {
                contract_id: target_contract.contract_id,
                revision: target_contract.revision,
            },
        },
        WorkRelationKindDto::ResultDependency { result_digest } => {
            WorkRelationKind::ResultDependency { result_digest }
        }
    }
}

fn mutation_result(command: &WorkCommandResult) -> Result<WorkRunMutationResult, RpcError> {
    Ok(WorkRunMutationResult {
        disposition: match command.disposition {
            WorkCommandDisposition::Committed => WorkRunCommandDispositionDto::Committed,
            WorkCommandDisposition::Replayed => WorkRunCommandDispositionDto::Replayed,
        },
        work_run: work_run_projection::work_run(&command.work_run)
            .map_err(work_projection_error)?,
    })
}

fn require_work_coordination_host(connection: &ConnectionState) -> Result<(), RpcError> {
    if connection.supports_work_coordination_host() {
        Ok(())
    } else {
        Err(RpcError::new(
            -32073,
            AppServerErrorName::PermissionRequired,
        ))
    }
}

fn work_coordination_unavailable() -> RpcError {
    RpcError::new(-32090, AppServerErrorName::WorkCoordinationUnavailable)
}

fn invalid_topology() -> RpcError {
    RpcError::new(-32602, AppServerErrorName::InvalidParams)
}

fn work_projection_error(_error: String) -> RpcError {
    RpcError::new(-32093, AppServerErrorName::WorkCoordinationOperationFailed)
}

fn work_coordination_error(error: WorkCoordinationError) -> RpcError {
    match error {
        WorkCoordinationError::NotFound(_) => {
            RpcError::new(-32091, AppServerErrorName::WorkCoordinationNotFound)
        }
        WorkCoordinationError::RevisionConflict { .. } => {
            RpcError::new(-32092, AppServerErrorName::WorkCoordinationRevisionConflict)
        }
        WorkCoordinationError::CommandConflict => {
            RpcError::new(-32012, AppServerErrorName::CommandConflict)
        }
        WorkCoordinationError::InvalidInput(_)
        | WorkCoordinationError::AlreadyExists(_)
        | WorkCoordinationError::WorkRunClosed
        | WorkCoordinationError::InvalidTransition(_)
        | WorkCoordinationError::ThreadBusy { .. }
        | WorkCoordinationError::Storage(_) => {
            RpcError::new(-32093, AppServerErrorName::WorkCoordinationOperationFailed)
        }
    }
}
