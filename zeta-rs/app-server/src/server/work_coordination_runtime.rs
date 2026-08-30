use super::turn_changes_runtime::TurnChangesRuntime;
use super::update_broker::UpdateBroker;
use super::work_run_projection;
use super::work_wait::next_wait_resolution;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use zeta_app_server_protocol::protocol::work_runs::WorkRunChanged;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkRunId;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkAttemptWorkspace;
use zeta_work_coordination::WorkCommandDisposition;
use zeta_work_coordination::WorkCommandResult;
use zeta_work_coordination::WorkCoordinationError;
use zeta_work_coordination::WorkCoordinator;
use zeta_work_coordination::WorkIntegrationStatus;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;
use zeta_work_coordination::WorkRunStore;
use zeta_work_coordination::WorkVerificationStatus;

const MAX_RECONCILE_TRANSITIONS: usize = 1_024;

/// App Server owner of coordination persistence and its physical WorkAttempt effects.
pub(super) struct WorkCoordinationRuntime {
    coordinator: WorkCoordinator,
    workspace_host: RwLock<Option<Arc<TurnChangesRuntime>>>,
    reconcile_gate: Mutex<()>,
    updates: Arc<UpdateBroker>,
}

impl WorkCoordinationRuntime {
    pub(super) fn new(store: Arc<dyn WorkRunStore>, updates: Arc<UpdateBroker>) -> Self {
        Self {
            coordinator: WorkCoordinator::new(store),
            workspace_host: RwLock::new(None),
            reconcile_gate: Mutex::new(()),
            updates,
        }
    }

    pub(super) fn list(&self) -> Result<Vec<WorkRun>, WorkCoordinationError> {
        self.coordinator.list()
    }

    pub(super) fn read(&self, work_run_id: &WorkRunId) -> Result<WorkRun, WorkCoordinationError> {
        self.coordinator.read(work_run_id)
    }

    #[cfg(test)]
    pub(super) fn apply_state_for_test(
        &self,
        request: WorkRunCommandRequest,
    ) -> Result<WorkCommandResult, WorkCoordinationError> {
        self.coordinator.apply(request)
    }

    pub(super) fn apply(
        &self,
        request: WorkRunCommandRequest,
    ) -> Result<WorkCommandResult, WorkCoordinationError> {
        let seal_barrier = if matches!(request.command, WorkRunCommand::SealAttempt { .. }) {
            if let Some(replayed) = self.coordinator.replay(&request)? {
                return Ok(replayed);
            }
            Some(self.validate_attempt_result(&request)?)
        } else {
            None
        };
        let work_run_id = request.work_run_id.clone();
        let result = match self.coordinator.apply(request) {
            Ok(result) => result,
            Err(error) => {
                if let Some((host, thread_id)) = seal_barrier {
                    host.release_attempt_result_barrier(&thread_id);
                }
                return Err(error);
            }
        };
        self.reconcile(&work_run_id)?;
        Ok(result)
    }

    /// Starts verification from an exact Attempt selection while keeping every evidence fact
    /// host-derived and retry-safe.
    pub(super) fn request_verification(
        &self,
        command_id: CommandId,
        work_run_id: WorkRunId,
        expected_revision: u64,
        selected_attempt_ids: BTreeSet<WorkAttemptId>,
    ) -> Result<WorkCommandResult, WorkCoordinationError> {
        if let Some(receipt) = self.coordinator.command_receipt(&command_id)? {
            let replayed_selection = match &receipt.request.command {
                WorkRunCommand::BeginVerification { input } => input
                    .ordered_results
                    .iter()
                    .map(|result| result.attempt_id.clone())
                    .collect::<BTreeSet<_>>(),
                _ => return Err(WorkCoordinationError::CommandConflict),
            };
            if receipt.request.work_run_id != work_run_id
                || receipt.request.expected_revision != expected_revision
                || replayed_selection != selected_attempt_ids
            {
                return Err(WorkCoordinationError::CommandConflict);
            }
            self.reconcile(&work_run_id)?;
            return Ok(WorkCommandResult {
                work_run: self.coordinator.read(&work_run_id)?,
                disposition: WorkCommandDisposition::Replayed,
            });
        }
        let run = self.coordinator.read(&work_run_id)?;
        if run.revision != expected_revision {
            return Err(WorkCoordinationError::RevisionConflict {
                expected: expected_revision,
                actual: run.revision,
            });
        }
        let host = self
            .workspace_host
            .read()
            .map_err(|_| WorkCoordinationError::Storage("workspace host lock poisoned".into()))?
            .clone()
            .ok_or_else(|| {
                WorkCoordinationError::InvalidTransition(
                    "WorkVerification evidence authority is unavailable".into(),
                )
            })?;
        let input = host
            .prepare_work_verification(&run, &selected_attempt_ids)
            .map_err(WorkCoordinationError::InvalidTransition)?;
        let result = self.apply(WorkRunCommandRequest {
            command_id,
            work_run_id: work_run_id.clone(),
            expected_revision,
            command: WorkRunCommand::BeginVerification { input },
        })?;
        Ok(WorkCommandResult {
            work_run: self.coordinator.read(&work_run_id)?,
            disposition: result.disposition,
        })
    }

    /// Queues publication of one exact verified result set. Preparation and publication facts are
    /// derived by the host and persisted one root at a time by reconciliation.
    pub(super) fn request_integration(
        &self,
        command_id: CommandId,
        work_run_id: WorkRunId,
        expected_revision: u64,
        verification_key: ContentDigest,
    ) -> Result<WorkCommandResult, WorkCoordinationError> {
        if let Some(receipt) = self.coordinator.command_receipt(&command_id)? {
            let replayed_verification_key = match &receipt.request.command {
                WorkRunCommand::QueueIntegration { verification_key } => verification_key,
                _ => return Err(WorkCoordinationError::CommandConflict),
            };
            if receipt.request.work_run_id != work_run_id
                || receipt.request.expected_revision != expected_revision
                || replayed_verification_key != &verification_key
            {
                return Err(WorkCoordinationError::CommandConflict);
            }
            self.reconcile(&work_run_id)?;
            return Ok(WorkCommandResult {
                work_run: self.coordinator.read(&work_run_id)?,
                disposition: WorkCommandDisposition::Replayed,
            });
        }
        let run = self.coordinator.read(&work_run_id)?;
        if run.revision != expected_revision {
            return Err(WorkCoordinationError::RevisionConflict {
                expected: expected_revision,
                actual: run.revision,
            });
        }
        let host = self
            .workspace_host
            .read()
            .map_err(|_| WorkCoordinationError::Storage("workspace host lock poisoned".into()))?
            .clone()
            .ok_or_else(|| {
                WorkCoordinationError::InvalidTransition(
                    "WorkIntegration publication authority is unavailable".into(),
                )
            })?;
        host.validate_work_integration(&run, &verification_key)
            .map_err(WorkCoordinationError::InvalidTransition)?;
        let result = self.apply(WorkRunCommandRequest {
            command_id,
            work_run_id: work_run_id.clone(),
            expected_revision,
            command: WorkRunCommand::QueueIntegration { verification_key },
        })?;
        Ok(WorkCommandResult {
            work_run: self.coordinator.read(&work_run_id)?,
            disposition: result.disposition,
        })
    }

    fn validate_attempt_result(
        &self,
        request: &WorkRunCommandRequest,
    ) -> Result<(Arc<TurnChangesRuntime>, zeta_protocol::ThreadId), WorkCoordinationError> {
        let run = self.coordinator.read(&request.work_run_id)?;
        if run.revision != request.expected_revision {
            return Err(WorkCoordinationError::RevisionConflict {
                expected: request.expected_revision,
                actual: run.revision,
            });
        }
        let WorkRunCommand::SealAttempt {
            attempt_id,
            result_digest,
            change_set_ids,
            private_output_digest,
            external_effects_digest,
            external_effects_status,
        } = &request.command
        else {
            return Err(WorkCoordinationError::InvalidTransition(
                "attempt result validation requires a seal command".into(),
            ));
        };
        let host = self
            .workspace_host
            .read()
            .map_err(|_| WorkCoordinationError::Storage("workspace host lock poisoned".into()))?
            .clone()
            .ok_or_else(|| {
                WorkCoordinationError::InvalidTransition(
                    "WorkAttempt evidence authority is unavailable".into(),
                )
            })?;
        let thread_id = run
            .attempts
            .get(attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(attempt_id.to_string()))?
            .thread_id
            .clone();
        host.validate_attempt_result(
            &run,
            attempt_id,
            change_set_ids,
            private_output_digest,
            external_effects_digest,
            *external_effects_status,
            result_digest,
        )
        .map_err(WorkCoordinationError::InvalidTransition)?;
        Ok((host, thread_id))
    }

    pub(super) fn attach_workspace_host(
        &self,
        host: Arc<TurnChangesRuntime>,
    ) -> Result<(), WorkCoordinationError> {
        let mut current = self
            .workspace_host
            .write()
            .map_err(|_| WorkCoordinationError::Storage("workspace host lock poisoned".into()))?;
        if current.is_some() {
            return Err(WorkCoordinationError::Storage(
                "workspace host is already installed".into(),
            ));
        }
        *current = Some(host);
        drop(current);
        for run in self.coordinator.list()? {
            self.reconcile(&run.work_run_id)?;
        }
        Ok(())
    }

    fn reconcile(&self, work_run_id: &WorkRunId) -> Result<(), WorkCoordinationError> {
        let Some(host) = self
            .workspace_host
            .read()
            .map_err(|_| WorkCoordinationError::Storage("workspace host lock poisoned".into()))?
            .clone()
        else {
            return Ok(());
        };
        let _guard = self
            .reconcile_gate
            .lock()
            .map_err(|_| WorkCoordinationError::Storage("reconcile lock poisoned".into()))?;
        for _ in 0..MAX_RECONCILE_TRANSITIONS {
            let run = self.coordinator.read(work_run_id)?;
            host.acknowledge_work_integration_receipts(&run)
                .map_err(WorkCoordinationError::Storage)?;
            let mut transition = None;
            for attempt in run.attempts.values() {
                match &attempt.workspace {
                    WorkAttemptWorkspace::Provisioning => {
                        transition = Some(
                            match host.ensure_work_attempt_workspace(work_run_id, attempt) {
                                Ok((roots, private_output_dir_id)) => {
                                    WorkRunCommand::RecordAttemptWorkspaceReady {
                                        attempt_id: attempt.attempt_id.clone(),
                                        roots,
                                        private_output_dir_id,
                                    }
                                }
                                Err(reason) => WorkRunCommand::FailAttemptWorkspace {
                                    attempt_id: attempt.attempt_id.clone(),
                                    reason,
                                },
                            },
                        );
                        break;
                    }
                    WorkAttemptWorkspace::Ready { .. } => {
                        let active = matches!(
                            attempt.execution_status,
                            WorkAttemptExecutionStatus::Exploring
                                | WorkAttemptExecutionStatus::Writing
                        );
                        let result = if active {
                            host.activate_work_attempt_workspace(work_run_id, attempt)
                        } else {
                            host.deactivate_work_attempt_workspace(work_run_id, attempt)
                        };
                        if let Err(message) = result
                            && active
                        {
                            transition = Some(WorkRunCommand::InterruptAttempt {
                                attempt_id: attempt.attempt_id.clone(),
                                message: format!(
                                    "WorkAttempt execution boundary could not be enforced: {message}"
                                ),
                            });
                            break;
                        }
                    }
                    WorkAttemptWorkspace::Failed { .. } => {
                        host.deactivate_work_attempt_workspace(work_run_id, attempt)
                            .map_err(WorkCoordinationError::Storage)?;
                    }
                }
            }
            if transition.is_none() {
                transition = next_wait_resolution(&run)?;
            }
            if transition.is_none()
                && let Some(verification) = run
                    .verifications
                    .values()
                    .find(|verification| verification.status == WorkVerificationStatus::Verifying)
            {
                let execution = host.execute_work_verification(&run, verification);
                transition = Some(WorkRunCommand::FinishVerification {
                    verification_key: verification.verification_key.clone(),
                    conclusion: execution.conclusion,
                    checks: execution.checks,
                    reason: execution.reason,
                });
            }
            if transition.is_none() {
                for integration in run.integrations.values() {
                    match integration.status {
                        WorkIntegrationStatus::Queued => {
                            if let Some(root) = integration.roots.iter().find(|root| {
                                root.status
                                    == zeta_work_coordination::IntegrationRootStatus::Pending
                            }) {
                                transition = Some(
                                    match host.prepare_work_integration_root(
                                        &run,
                                        integration,
                                        root,
                                    ) {
                                        Ok(artifact) => {
                                            WorkRunCommand::RecordIntegrationRootPrepared {
                                                integration_key: integration
                                                    .integration_key
                                                    .clone(),
                                                generation: integration.generation,
                                                root_id: root.root_id.clone(),
                                                artifact,
                                            }
                                        }
                                        Err(failure) => WorkRunCommand::FailIntegration {
                                            integration_key: integration.integration_key.clone(),
                                            generation: integration.generation,
                                            kind: failure.kind,
                                            reason: failure.reason,
                                        },
                                    },
                                );
                            } else {
                                transition = Some(WorkRunCommand::BeginIntegration {
                                    integration_key: integration.integration_key.clone(),
                                    generation: integration.generation,
                                });
                            }
                        }
                        WorkIntegrationStatus::Integrating => {
                            if let Some(root) = integration.roots.iter().find(|root| {
                                root.status
                                    != zeta_work_coordination::IntegrationRootStatus::Published
                            }) {
                                transition = Some(
                                    match host.publish_work_integration_root(
                                        &run,
                                        integration,
                                        root,
                                    ) {
                                        Ok(receipt_digest) => {
                                            WorkRunCommand::RecordIntegrationRootPublished {
                                                integration_key: integration
                                                    .integration_key
                                                    .clone(),
                                                generation: integration.generation,
                                                root_id: root.root_id.clone(),
                                                receipt_digest,
                                            }
                                        }
                                        Err(failure) => WorkRunCommand::FailIntegration {
                                            integration_key: integration.integration_key.clone(),
                                            generation: integration.generation,
                                            kind: failure.kind,
                                            reason: failure.reason,
                                        },
                                    },
                                );
                            }
                        }
                        WorkIntegrationStatus::Integrated
                        | WorkIntegrationStatus::Partial
                        | WorkIntegrationStatus::Conflict
                        | WorkIntegrationStatus::Failed => {}
                    }
                    if transition.is_some() {
                        break;
                    }
                }
            }
            let Some(command) = transition else {
                return Ok(());
            };
            let command_id = runtime_command_id(work_run_id, run.revision, &command)?;
            let attempt_id = command_attempt_id(&command).cloned();
            match self.coordinator.apply(WorkRunCommandRequest {
                command_id,
                work_run_id: work_run_id.clone(),
                expected_revision: run.revision,
                command,
            }) {
                Ok(result) => {
                    if result.disposition == WorkCommandDisposition::Committed {
                        self.publish(&result.work_run)?;
                    }
                }
                Err(WorkCoordinationError::RevisionConflict { .. }) => continue,
                Err(WorkCoordinationError::InvalidTransition(_))
                    if attempt_id.is_some_and(|attempt_id| {
                        self.coordinator
                            .read(work_run_id)
                            .ok()
                            .and_then(|run| run.attempts.get(&attempt_id).cloned())
                            .is_some_and(|attempt| {
                                !matches!(attempt.workspace, WorkAttemptWorkspace::Provisioning)
                            })
                    }) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        Err(WorkCoordinationError::Storage(
            "WorkRun reconciliation exceeded its finite transition bound".into(),
        ))
    }

    fn publish(&self, run: &WorkRun) -> Result<(), WorkCoordinationError> {
        let work_run =
            work_run_projection::work_run(run).map_err(WorkCoordinationError::Storage)?;
        self.updates
            .publish_work_run_changed(WorkRunChanged { work_run });
        Ok(())
    }
}

fn runtime_command_id(
    work_run_id: &WorkRunId,
    revision: u64,
    command: &WorkRunCommand,
) -> Result<CommandId, WorkCoordinationError> {
    let encoded = serde_json::to_vec(&(1_u32, work_run_id, revision, command))
        .map_err(|error| WorkCoordinationError::Storage(error.to_string()))?;
    let digest = ContentDigest::sha256(&encoded)
        .to_string()
        .replace(':', "-");
    CommandId::new(format!("work-runtime-{digest}"))
        .map_err(|error| WorkCoordinationError::Storage(error.to_string()))
}

fn command_attempt_id(command: &WorkRunCommand) -> Option<&WorkAttemptId> {
    match command {
        WorkRunCommand::RecordAttemptWorkspaceReady { attempt_id, .. }
        | WorkRunCommand::FailAttemptWorkspace { attempt_id, .. }
        | WorkRunCommand::InterruptAttempt { attempt_id, .. } => Some(attempt_id),
        _ => None,
    }
}
