use super::AppServer;
use super::RpcError;
use super::issue_assignment::now;
use super::issue_operations::issue_error;
use super::turn_changes_runtime::TurnChangesRuntime;
use super::work_coordination_runtime::WorkCoordinationRuntime;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use zeta_core::SequenceExpectation;
use zeta_core::StartThreadRequest;
use zeta_core::StartTurnRequest;
use zeta_core::TurnExecutionBackend;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_state::IssueAssignmentCommand;
use zeta_turn_changes::TurnChangeStore;
use zeta_work_coordination::IssueAssignment;
use zeta_work_coordination::IssueAssignmentPlan;
use zeta_work_coordination::IssueOwnership;
use zeta_work_coordination::IssueSyncState;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkContractDraft;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;

/// Owns the scheduler lifetime; model execution remains owned by the normal Turn backend.
pub(super) struct IssueExecutionRuntime {
    context: Arc<IssueExecutionContext>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    heartbeat: Option<std::thread::JoinHandle<()>>,
    synchronizer: Option<std::thread::JoinHandle<()>>,
}

struct IssueExecutionContext {
    stop: Arc<AtomicBool>,
    sync_cancel: zeta_async_utils::CancellationSource,
    store: Arc<zeta_state::SqliteIssueAssignmentStore>,
    tasks: Arc<zeta_state::SqliteIssueTaskStore>,
    changes: Arc<TurnChangesRuntime>,
    work: Arc<WorkCoordinationRuntime>,
    backend: Arc<super::turn_backend_router::TurnBackendHandle>,
    agents: Arc<zeta_core::MultiAgentCoordinator>,
    environment: Arc<std::sync::RwLock<super::environment_runtime::EnvRuntime>>,
    repository: String,
    updates: Arc<super::update_broker::UpdateBroker>,
    notices: Mutex<BTreeMap<String, String>>,
    gate: Mutex<()>,
    owned: Mutex<BTreeMap<String, u64>>,
    next_sync: Mutex<BTreeMap<String, std::time::Instant>>,
    auto_checked: Mutex<Option<std::time::Instant>>,
}

impl Drop for IssueExecutionRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.context.sync_cancel.cancel();
        if let Some(synchronizer) = self.synchronizer.take() {
            synchronizer.thread().unpark();
            let _ = synchronizer.join();
        }
        if let Some(heartbeat) = self.heartbeat.take() {
            heartbeat.thread().unpark();
            let _ = heartbeat.join();
        }
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

impl AppServer {
    pub(super) fn prepare_issue_assignment_thread(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<zeta_core::ThreadSnapshot, RpcError> {
        let runtime = self.turn_changes_runtime()?;
        let store = self
            .issue_tasks
            .as_deref()
            .ok_or_else(|| issue_error("Issue task store unavailable".into()))?;
        let task = match store.read_command(&assignment.id).map_err(issue_error)? {
            Some(task) => task,
            None => {
                let repository = zeta_github::Repository::new(
                    assignment.repository.host.clone(),
                    assignment.repository.owner.clone(),
                    assignment.repository.name.clone(),
                )
                .map_err(issue_error)?;
                let github = zeta_github::GitHub::default();
                let mut issues = Vec::new();
                for issue in &assignment.item.issues {
                    issues.push(
                        runtime
                            .worktree_runtime
                            .block_on(github.issue(&repository, issue.number))
                            .map_err(issue_error)?,
                    );
                }
                let tree = runtime
                    .worktree_runtime
                    .block_on(async {
                        let git = zeta_git::GitClient::system();
                        let checkout = git.open_repository(&runtime.dir_root).await?;
                        git.resolve_tree(&checkout, &assignment.base_commit).await
                    })
                    .map_err(|error| issue_error(error.to_string()))?;
                zeta_github::IssueTask {
                    command_id: assignment.id.clone(),
                    fingerprint: ContentDigest::sha256(assignment.id.as_bytes()).to_string(),
                    session_id: String::new(),
                    repository,
                    issues,
                    source_root: runtime.dir_root.clone(),
                    start_commit: assignment.base_commit.clone(),
                    start_tree: tree.as_str().into(),
                    branch: assignment.branch.clone(),
                    target_branch: assignment.target_branch.clone(),
                    read_at: now()?,
                    pull_request: None,
                }
            }
        };
        let binder = super::issue_tasks::IssueBinder {
            runtime: &runtime,
            store,
            task: &task,
        };
        let created = self
            .threads
            .start_thread(
                &binder,
                StartThreadRequest {
                    command_id: command(&assignment.id)?,
                    title: assignment.item.objective.clone(),
                },
            )
            .map_err(super::core_error)?;
        self.updates.bind_session_scope(created.session_id.clone());
        self.threads
            .install_session_extensions(
                created.session_id.clone(),
                Arc::clone(&self.agent_extensions),
            )
            .map_err(super::core_error)?;
        let latest = self
            .issue_assignment_store()?
            .read(&assignment.id)
            .map_err(issue_error)?;
        if latest.thread_id.is_none() {
            self.issue_assignment_store()?
                .apply(
                    &format!("{}-thread", assignment.id),
                    &assignment.id,
                    latest.revision,
                    IssueAssignmentCommand::RecordThread {
                        thread_id: created.thread_id.clone(),
                    },
                    now()?,
                )
                .map_err(issue_error)?;
        }
        let latest = self
            .issue_assignment_store()?
            .read(&assignment.id)
            .map_err(issue_error)?;
        if latest.agent_role.is_none() {
            let environment = self
                .env_runtime
                .read()
                .map_err(|_| issue_error("Environment lock poisoned".into()))?;
            let executor = &environment.turn_executor;
            let profile = executor
                .tool_profile_snapshot()
                .map_err(super::core_error)?;
            let (agents, instructions) = match &environment._dir_contributions {
                Some(source) => (
                    source.agent_snapshots_for(&created.session_id),
                    source.instruction_snapshots_for(&created.session_id),
                ),
                None => (Vec::new(), Vec::new()),
            };
            let selected = super::multi_agent_tools::agent_selection::resolve_agent_selection(
                (!assignment.item.agent.is_empty()).then_some(assignment.item.agent.as_str()),
                &assignment.item.objective,
                assignment.model.as_ref(),
                profile.tool_names,
                &[],
                &agents,
                &instructions,
            )
            .map_err(super::core_error)?;
            self.issue_assignment_store()?
                .apply(
                    &format!("{}-agent", assignment.id),
                    &assignment.id,
                    latest.revision,
                    IssueAssignmentCommand::FreezeAgent {
                        role: selected.role,
                        tools: selected
                            .capability_scope
                            .tools
                            .into_iter()
                            .filter(|tool| tool.as_str() != "create_goal")
                            .collect(),
                    },
                    now()?,
                )
                .map_err(issue_error)?;
        }
        Ok(created)
    }

    pub(super) fn start_issue_batch(
        &self,
        plan: &IssueAssignmentPlan,
        assignments: &[IssueAssignment],
    ) -> Result<(), RpcError> {
        let ready = assignments
            .iter()
            .filter(|assignment| assignment.sync_state == IssueSyncState::Synced)
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Ok(());
        }
        let work = self
            .work_coordination
            .as_ref()
            .ok_or_else(|| issue_error("Work coordination unavailable".into()))?;
        let run_id = WorkRunId::new(format!(
            "issue-batch-{}",
            ContentDigest::sha256(assignments[0].batch_id.as_bytes())
                .to_string()
                .replace(':', "-")
        ))
        .map_err(|error| issue_error(error.to_string()))?;
        let mut threads = Vec::new();
        for assignment in &ready {
            threads.push(self.prepare_issue_assignment_thread(assignment)?);
        }
        if matches!(
            work.read(&run_id),
            Err(zeta_work_coordination::WorkCoordinationError::NotFound(_))
        ) {
            let runtime = self.turn_changes_runtime()?;
            runtime
                .worktree_runtime
                .block_on(async {
                    let git = zeta_git::GitClient::system();
                    let repository = git.open_repository(&runtime.dir_root).await?;
                    git.create_branch_at(&repository, &batch_branch(&run_id), &plan.base_commit)
                        .await
                })
                .map_err(|error| issue_error(error.to_string()))?;
        }
        let mut run = match work.read(&run_id) {
            Ok(run) => run,
            Err(zeta_work_coordination::WorkCoordinationError::NotFound(_)) => apply(
                work,
                &run_id,
                0,
                &format!("{run_id}-create"),
                WorkRunCommand::Create {
                    objective: "Implement the accepted Issue assignment plan".into(),
                    acceptance_conditions: plan
                        .items
                        .iter()
                        .flat_map(|item| item.acceptance_conditions.clone())
                        .collect(),
                    exclusions: Vec::new(),
                    root_participant: WorkParticipant {
                        session_id: threads[0].session_id.clone(),
                        thread_id: threads[0].thread_id.clone(),
                        relation: WorkParticipantRelation::Root,
                    },
                },
            )?,
            Err(error) => return Err(issue_error(error.to_string())),
        };
        for thread in &threads {
            if !run.participants.contains_key(&thread.thread_id) {
                run = apply(
                    work,
                    &run_id,
                    run.revision,
                    &format!("{run_id}-{}-join", thread.thread_id),
                    WorkRunCommand::AddParticipant {
                        participant: WorkParticipant {
                            session_id: thread.session_id.clone(),
                            thread_id: thread.thread_id.clone(),
                            relation: WorkParticipantRelation::Root,
                        },
                    },
                )?;
            }
        }
        for (assignment, thread) in ready.into_iter().zip(threads) {
            let latest = self
                .issue_assignment_store()?
                .read(&assignment.id)
                .map_err(issue_error)?;
            if latest.work_run_id.is_none() {
                self.issue_assignment_store()?
                    .apply(
                        &format!("{}-work", assignment.id),
                        &assignment.id,
                        latest.revision,
                        IssueAssignmentCommand::RecordWork {
                            thread_id: thread.thread_id,
                            work_run_id: run_id.clone(),
                            attempt_id: WorkAttemptId::new(format!(
                                "{}-a{}",
                                assignment.id, latest.epoch
                            ))
                            .map_err(|error| issue_error(error.to_string()))?,
                        },
                        now()?,
                    )
                    .map_err(issue_error)?;
            }
        }
        self.issue_execution_runtime(&plan.repository.key())?
            .worker
            .as_ref()
            .expect("scheduler thread exists")
            .thread()
            .unpark();
        Ok(())
    }

    pub(crate) fn recover_issue_assignments(&self) -> Result<(), String> {
        let Some(store) = &self.issue_assignments else {
            return Ok(());
        };
        let changes = match self.turn_changes_runtime() {
            Ok(changes) => changes,
            Err(_) => return Ok(()),
        };
        let mut repository = store.auto_claim_for_directory(&changes.dir_root)?;
        for assignment in store.list_all()? {
            let Some(thread) = &assignment.thread_id else {
                continue;
            };
            let Some(tasks) = &self.issue_tasks else {
                continue;
            };
            if !tasks
                .read_session(thread.as_str())?
                .is_some_and(|task| task.source_root == changes.dir_root)
            {
                continue;
            }
            if matches!(
                assignment.ownership,
                IssueOwnership::Held
                    | IssueOwnership::Releasing
                    | IssueOwnership::Transferring
                    | IssueOwnership::Cancelled
            ) {
                repository = Some(assignment.repository.key());
            }
            if assignment.ownership != IssueOwnership::Held || assignment.lease_until.is_none() {
                continue;
            }
            self.multi_agent
                .cancel_descendants(thread)
                .map_err(|error| error.to_string())?;
            let snapshot = self
                .threads
                .read_thread(thread)
                .map_err(|error| error.to_string())?;
            if snapshot
                .goal
                .as_ref()
                .is_some_and(|goal| goal.status == zeta_protocol::ThreadGoalStatus::Active)
            {
                self.threads
                    .set_goal(
                        thread,
                        zeta_core::SetGoalRequest {
                            status: Some(zeta_protocol::ThreadGoalStatus::Paused),
                            ..Default::default()
                        },
                    )
                    .map_err(|error| error.to_string())?;
            }
            if let (Some(work), Some(run_id), Some(attempt_id)) = (
                &self.work_coordination,
                &assignment.work_run_id,
                &assignment.attempt_id,
            ) {
                let run = work.read(run_id).map_err(|error| error.to_string())?;
                if snapshot
                    .turns
                    .last()
                    .is_some_and(|turn| turn.status == zeta_protocol::TurnStatus::Completed)
                    && snapshot
                        .goal
                        .as_ref()
                        .is_none_or(|goal| goal.status == zeta_protocol::ThreadGoalStatus::Complete)
                {
                    if run.attempts.get(attempt_id).is_some_and(|attempt| {
                        attempt.execution_status == WorkAttemptExecutionStatus::Sealed
                    }) {
                        store.apply(
                            &format!("{attempt_id}-recover-finish"),
                            &assignment.id,
                            assignment.revision,
                            IssueAssignmentCommand::FinishExecution {
                                epoch: assignment.epoch,
                            },
                            super::issue_assignment::now().map_err(|error| error.to_string())?,
                        )?;
                        continue;
                    }
                    if let Ok(result) = changes.derive_attempt_result(&run, attempt_id) {
                        apply(
                            work,
                            run_id,
                            run.revision,
                            &format!("{attempt_id}-recover-seal"),
                            WorkRunCommand::SealAttempt {
                                attempt_id: attempt_id.clone(),
                                result_digest: result.result_digest,
                                change_set_ids: result.change_set_ids,
                                private_output_digest: result.private_output_digest,
                                external_effects_digest: result.external_effects_digest,
                                external_effects_status: result.external_effects_status,
                            },
                        )
                        .map_err(|error| error.to_string())?;
                        store.apply(
                            &format!("{attempt_id}-recover-finish"),
                            &assignment.id,
                            assignment.revision,
                            IssueAssignmentCommand::FinishExecution {
                                epoch: assignment.epoch,
                            },
                            super::issue_assignment::now().map_err(|error| error.to_string())?,
                        )?;
                        continue;
                    }
                }
                if run.attempts.get(attempt_id).is_some_and(|attempt| {
                    matches!(
                        attempt.execution_status,
                        WorkAttemptExecutionStatus::Writing
                            | WorkAttemptExecutionStatus::Exploring
                            | WorkAttemptExecutionStatus::Waiting
                    )
                }) {
                    apply(
                        work,
                        run_id,
                        run.revision,
                        &format!("{attempt_id}-host-interrupted"),
                        WorkRunCommand::InterruptAttempt {
                            attempt_id: attempt_id.clone(),
                            message: "Issue execution was interrupted by a host restart".into(),
                        },
                    )
                    .map_err(|error| error.to_string())?;
                }
            }
            store.apply(
                &format!("{}-recover-stop-{}", assignment.id, assignment.epoch),
                &assignment.id,
                assignment.revision,
                IssueAssignmentCommand::Stop {
                    epoch: assignment.epoch,
                },
                super::issue_assignment::now().map_err(|error| error.to_string())?,
            )?;
        }
        if let Some(repository) = repository {
            self.issue_execution_runtime(&repository)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(super) fn ensure_issue_repository_scheduler(
        &self,
        repository: &zeta_work_coordination::IssueRepositoryIdentity,
    ) -> Result<(), RpcError> {
        let runtime = self.issue_execution_runtime(&repository.key())?;
        *runtime
            .context
            .auto_checked
            .lock()
            .map_err(|_| issue_error("Auto-claim timer lock poisoned".into()))? = None;
        runtime
            .synchronizer
            .as_ref()
            .expect("Issue synchronization worker exists")
            .thread()
            .unpark();
        Ok(())
    }

    pub(super) fn ensure_issue_scheduler(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<(), RpcError> {
        let runtime = self.issue_execution_runtime(&assignment.repository.key())?;
        runtime
            .context
            .next_sync
            .lock()
            .map_err(|_| issue_error("Issue sync timer lock poisoned".into()))?
            .remove(&format!("audit:{}", assignment.id));
        runtime
            .worker
            .as_ref()
            .expect("Issue scheduler exists")
            .thread()
            .unpark();
        runtime
            .synchronizer
            .as_ref()
            .expect("Issue synchronizer exists")
            .thread()
            .unpark();
        Ok(())
    }

    fn issue_execution_runtime(
        &self,
        repository_key: &str,
    ) -> Result<&IssueExecutionRuntime, RpcError> {
        if let Some(runtime) = self.issue_execution.get() {
            if runtime.context.repository != repository_key {
                return Err(issue_error(
                    "Repository changed; restart its coordinator before assigning work".into(),
                ));
            }
            return Ok(runtime);
        }
        let _init = self
            .issue_execution_init
            .lock()
            .map_err(|_| issue_error("Issue scheduler initialization lock poisoned".into()))?;
        if let Some(runtime) = self.issue_execution.get() {
            return Ok(runtime);
        }
        let stop = Arc::new(AtomicBool::new(false));
        let context = Arc::new(IssueExecutionContext {
            stop: stop.clone(),
            sync_cancel: zeta_async_utils::CancellationSource::new(),
            tasks: self
                .issue_tasks
                .clone()
                .ok_or_else(|| issue_error("Issue task store unavailable".into()))?,
            store: self
                .issue_assignments
                .clone()
                .ok_or_else(|| issue_error("Issue assignment store unavailable".into()))?,
            changes: self.turn_changes_runtime()?,
            work: self
                .work_coordination
                .clone()
                .ok_or_else(|| issue_error("Work coordination unavailable".into()))?,
            backend: self.turn_backend.clone(),
            agents: self.multi_agent.clone(),
            environment: self.env_runtime.clone(),
            repository: repository_key.to_owned(),
            updates: self.updates.clone(),
            notices: Mutex::new(BTreeMap::new()),
            gate: Mutex::new(()),
            owned: Mutex::new(BTreeMap::new()),
            next_sync: Mutex::new(BTreeMap::new()),
            auto_checked: Mutex::new(None),
        });
        let worker_context = context.clone();
        let worker_stop = stop.clone();
        let worker = std::thread::Builder::new()
            .name("zeta-issue-scheduler".into())
            .spawn(move || {
                while !worker_stop.load(Ordering::Acquire) {
                    if let Err(error) = worker_context.notify_changes() {
                        log::warn!("Issue notifications: {error}");
                    }
                    if let Err(error) = worker_context.tick() {
                        log::warn!("Issue scheduler: {error:?}");
                    }
                    std::thread::park_timeout(Duration::from_secs(1));
                }
            })
            .map_err(|error| issue_error(error.to_string()))?;
        let mut runtime = IssueExecutionRuntime {
            context,
            stop,
            worker: Some(worker),
            heartbeat: None,
            synchronizer: None,
        };
        let heartbeat_context = runtime.context.clone();
        let heartbeat_stop = runtime.stop.clone();
        runtime.heartbeat = Some(
            std::thread::Builder::new()
                .name("zeta-issue-heartbeat".into())
                .spawn(move || {
                    while !heartbeat_stop.load(Ordering::Acquire) {
                        if let Err(error) = heartbeat_context.heartbeat() {
                            log::warn!("Issue heartbeat: {error:?}");
                        }
                        std::thread::park_timeout(Duration::from_secs(15));
                    }
                })
                .map_err(|error| issue_error(error.to_string()))?,
        );
        let sync_context = runtime.context.clone();
        let sync_stop = runtime.stop.clone();
        runtime.synchronizer = Some(
            std::thread::Builder::new()
                .name("zeta-issue-sync".into())
                .spawn(move || {
                    while !sync_stop.load(Ordering::Acquire) {
                        if let Err(error) = sync_context.auto_claim() {
                            log::warn!("Issue automatic claiming: {error}");
                        }
                        if let Err(error) = sync_context.audit_remote() {
                            log::warn!("Issue remote audit: {error}");
                        }
                        if let Err(error) = sync_context.sync_pending() {
                            log::warn!("Issue sync: {error}");
                        }
                        std::thread::park_timeout(Duration::from_secs(1));
                    }
                })
                .map_err(|error| issue_error(error.to_string()))?,
        );
        let _ = self.issue_execution.set(runtime);
        Ok(self
            .issue_execution
            .get()
            .expect("Issue scheduler initialized"))
    }

    pub(super) fn interrupt_issue_assignment(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<(), RpcError> {
        let executor = self.issue_execution_runtime(&assignment.repository.key())?;
        let _guard = executor
            .context
            .gate
            .lock()
            .map_err(|_| issue_error("Issue scheduler lock poisoned".into()))?;
        executor.context.interrupt(assignment)
    }

    pub(super) fn resume_issue_assignment(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<(), RpcError> {
        let mut prepared_claim = None;
        if assignment.ownership == IssueOwnership::Unclaimed {
            let (_, repository, _) = self.issue_repository_identity()?;
            let config = self
                .config
                .as_ref()
                .ok_or_else(|| issue_error("Issue configuration unavailable".into()))?
                .read_snapshot()
                .map_err(|error| issue_error(error.to_string()))?;
            let workflow = config
                .values
                .issues
                .repositories
                .get(&repository.key())
                .cloned()
                .ok_or_else(|| {
                    issue_error("Configure this repository's Issue workflow first".into())
                })?;
            prepared_claim = Some(
                self.issue_assignment_store()?
                    .apply(
                        &format!("{}-claim-{}", assignment.id, assignment.revision),
                        &assignment.id,
                        assignment.revision,
                        IssueAssignmentCommand::ClaimPrepared {
                            workflow,
                            config_revision: config.revision.get(),
                        },
                        now()?,
                    )
                    .map_err(issue_error)?,
            );
        }
        let assignment = prepared_claim.as_ref().unwrap_or(assignment);
        if assignment.work_run_id.is_none() {
            let prepared = self.prepare_issue_assignment(assignment)?;
            let siblings = self
                .issue_assignment_store()?
                .list(&assignment.repository.key())
                .map_err(issue_error)?
                .into_iter()
                .filter(|item| {
                    item.batch_id == assignment.batch_id && item.ownership == IssueOwnership::Held
                })
                .collect::<Vec<_>>();
            let plan = IssueAssignmentPlan {
                repository: prepared.repository.clone(),
                workflow: prepared.workflow.clone(),
                config_revision: 0,
                model: prepared.model.clone(),
                planning_tokens: prepared.planning_tokens,
                base_commit: prepared.base_commit.clone(),
                target_branch: prepared.target_branch.clone(),
                items: siblings.iter().map(|item| item.item.clone()).collect(),
            };
            self.start_issue_batch(&plan, &siblings)?;
        }
        if let (Some(run_id), Some(attempt_id)) = (&assignment.work_run_id, &assignment.attempt_id)
        {
            let work = self
                .work_coordination
                .as_ref()
                .ok_or_else(|| issue_error("Work coordination unavailable".into()))?;
            let run = work
                .read(run_id)
                .map_err(|error| issue_error(error.to_string()))?;
            if run.attempts.get(attempt_id).is_some_and(|attempt| {
                attempt.execution_status == WorkAttemptExecutionStatus::Sealed
            }) {
                self.issue_assignment_store()?
                    .apply(
                        &format!("{}-resume-result-{}", assignment.id, assignment.revision),
                        &assignment.id,
                        assignment.revision,
                        IssueAssignmentCommand::FinishExecution {
                            epoch: assignment.epoch,
                        },
                        now()?,
                    )
                    .map_err(issue_error)?;
                self.ensure_issue_scheduler(assignment)?;
                return Ok(());
            }
        }
        let executor = self.issue_execution_runtime(&assignment.repository.key())?;
        let _guard = executor
            .context
            .gate
            .lock()
            .map_err(|_| issue_error("Issue scheduler lock poisoned".into()))?;
        let latest = self
            .issue_assignment_store()?
            .read(&assignment.id)
            .map_err(issue_error)?;
        if latest.lease_until.is_some() {
            executor.context.stop(&latest)?;
        }
        let latest = self
            .issue_assignment_store()?
            .read(&assignment.id)
            .map_err(issue_error)?;
        self.issue_assignment_store()?
            .apply(
                &format!("{}-queue-{}", latest.id, latest.revision),
                &latest.id,
                latest.revision,
                IssueAssignmentCommand::Queue,
                now()?,
            )
            .map_err(issue_error)?;
        if let Some(thread) = &latest.thread_id {
            if self
                .threads
                .get_goal(thread)
                .map_err(super::core_error)?
                .is_some_and(|goal| goal.status == zeta_protocol::ThreadGoalStatus::Paused)
            {
                self.threads
                    .set_goal(
                        thread,
                        zeta_core::SetGoalRequest {
                            status: Some(zeta_protocol::ThreadGoalStatus::Active),
                            ..Default::default()
                        },
                    )
                    .map_err(super::core_error)?;
            }
        }
        Ok(())
    }

    pub(super) fn verify_issue_assignment(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<(), RpcError> {
        let work = self
            .work_coordination
            .as_ref()
            .ok_or_else(|| issue_error("Work coordination unavailable".into()))?;
        let run_id = assignment
            .work_run_id
            .as_ref()
            .ok_or_else(|| issue_error("Issue has no execution result".into()))?;
        let run = work
            .read(run_id)
            .map_err(|error| issue_error(error.to_string()))?;
        let attempt_id = assignment
            .attempt_id
            .clone()
            .ok_or_else(|| issue_error("Issue has no execution attempt".into()))?;
        let changes = self.turn_changes_runtime()?;
        let mut selected = run
            .attempts
            .values()
            .filter(|attempt| {
                attempt.execution_status == WorkAttemptExecutionStatus::Sealed
                    && attempt.integration_status
                        != zeta_work_coordination::WorkAttemptIntegrationStatus::Integrated
            })
            .map(|attempt| attempt.attempt_id.clone())
            .collect::<BTreeSet<_>>();
        let mut run = run;
        let key = if selected.is_empty() {
            run.verifications
                .values()
                .find(|verification| {
                    verification.status == zeta_work_coordination::WorkVerificationStatus::Verified
                        && verification
                            .input
                            .ordered_results
                            .iter()
                            .any(|result| result.attempt_id == attempt_id)
                })
                .map(|verification| verification.verification_key.clone())
                .ok_or_else(|| issue_error("Issue has no verified result".into()))?
        } else {
            if !selected.contains(&attempt_id) {
                selected.insert(attempt_id.clone());
            }
            let input = changes
                .prepare_work_verification(&run, &selected)
                .map_err(issue_error)?;
            let key = zeta_work_coordination::verification_key(run_id, &input)
                .map_err(|error| issue_error(error.to_string()))?;
            if run.verifications.contains_key(&key) {
                run = apply(
                    work,
                    run_id,
                    run.revision,
                    &format!("{}-verify-{}", assignment.id, run.revision),
                    WorkRunCommand::RestartVerification {
                        verification_key: key.clone(),
                    },
                )?;
            } else {
                let superseded = run
                    .verifications
                    .values()
                    .filter(|verification| {
                        verification.status != zeta_work_coordination::WorkVerificationStatus::Stale
                            && verification
                                .input
                                .ordered_results
                                .iter()
                                .any(|result| selected.contains(&result.attempt_id))
                    })
                    .map(|verification| verification.verification_key.clone())
                    .collect::<Vec<_>>();
                for old in superseded {
                    run = apply(
                        work,
                        run_id,
                        run.revision,
                        &format!("{}-supersede-{}", assignment.id, run.revision),
                        WorkRunCommand::MarkVerificationStale {
                            verification_key: old,
                            reason:
                                "Issue verification now includes the current result combination"
                                    .into(),
                        },
                    )?;
                }
                let outcome = work
                    .request_verification(
                        command(&format!("{}-verify-{}", assignment.id, run.revision))?,
                        run_id.clone(),
                        run.revision,
                        selected,
                    )
                    .map_err(|error| issue_error(error.to_string()))?;
                run = outcome.work_run;
            }
            if run.verifications[&key].status
                != zeta_work_coordination::WorkVerificationStatus::Verified
            {
                return Err(issue_error(
                    run.verifications[&key]
                        .reason
                        .clone()
                        .unwrap_or_else(|| "Independent verification did not pass".into()),
                ));
            }
            work.request_integration(
                command(&format!("{}-stage-{}", assignment.id, run.revision))?,
                run_id.clone(),
                run.revision,
                key.clone(),
            )
            .map_err(|error| issue_error(error.to_string()))?;
            key
        };
        self.prepare_issue_delivery(assignment, &key)?;
        Ok(())
    }

    pub(super) fn deliver_issue_assignment(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<(), RpcError> {
        self.publish_issue_delivery(assignment)?;
        Ok(())
    }
}

impl IssueExecutionContext {
    fn stop(&self, assignment: &IssueAssignment) -> Result<(), RpcError> {
        let current = self.store.read(&assignment.id).map_err(issue_error)?;
        if current.epoch != assignment.epoch {
            return Err(issue_error("Issue execution was superseded".into()));
        }
        self.store
            .apply(
                &format!("{}-stop-epoch-{}", current.id, current.epoch),
                &current.id,
                current.revision,
                IssueAssignmentCommand::Stop {
                    epoch: current.epoch,
                },
                now()?,
            )
            .map_err(issue_error)?;
        self.interrupt(&current)
    }

    fn interrupt(&self, assignment: &IssueAssignment) -> Result<(), RpcError> {
        if let Some(thread) = &assignment.thread_id {
            if self
                .changes
                .threads
                .get_goal(thread)
                .map_err(super::core_error)?
                .is_some_and(|goal| goal.status == zeta_protocol::ThreadGoalStatus::Active)
            {
                self.changes
                    .threads
                    .set_goal(
                        thread,
                        zeta_core::SetGoalRequest {
                            status: Some(zeta_protocol::ThreadGoalStatus::Paused),
                            ..Default::default()
                        },
                    )
                    .map_err(super::core_error)?;
            }
        }
        if let Some(thread_id) = &assignment.thread_id {
            self.agents
                .cancel_descendants(thread_id)
                .map_err(super::core_error)?;
        }
        if let (Some(run_id), Some(attempt_id)) = (&assignment.work_run_id, &assignment.attempt_id)
        {
            let run = self
                .work
                .read(run_id)
                .map_err(|error| issue_error(error.to_string()))?;
            if run.attempts.get(attempt_id).is_some_and(|attempt| {
                matches!(
                    attempt.execution_status,
                    WorkAttemptExecutionStatus::Writing
                        | WorkAttemptExecutionStatus::Exploring
                        | WorkAttemptExecutionStatus::Waiting
                )
            }) {
                apply(
                    &self.work,
                    run_id,
                    run.revision,
                    &format!("{}-stop-{}", assignment.id, assignment.epoch),
                    WorkRunCommand::InterruptAttempt {
                        attempt_id: attempt_id.clone(),
                        message: "Issue execution stopped by its owner".into(),
                    },
                )?;
            }
        }
        let mut owned = self
            .owned
            .lock()
            .map_err(|_| issue_error("Issue execution ownership lock poisoned".into()))?;
        if owned.get(&assignment.id) == Some(&assignment.epoch) {
            owned.remove(&assignment.id);
        }
        Ok(())
    }

    fn heartbeat(&self) -> Result<(), RpcError> {
        let owned = self
            .owned
            .lock()
            .map_err(|_| issue_error("Issue execution ownership lock poisoned".into()))?
            .clone();
        for (id, epoch) in owned {
            let assignment = self.store.read(&id).map_err(issue_error)?;
            if assignment.epoch != epoch
                || !assignment.auto_start
                || assignment.ownership != IssueOwnership::Held
            {
                continue;
            }
            if let Some(thread_id) = &assignment.thread_id {
                let session_id = zeta_protocol::SessionId::new(thread_id.as_str())
                    .map_err(|error| issue_error(error.to_string()))?;
                let usage = self
                    .changes
                    .threads
                    .list_session_threads(&session_id)
                    .map_err(super::core_error)?
                    .iter()
                    .fold(0_u64, |total, thread| {
                        total
                            .saturating_add(thread.usage.input_tokens.reported)
                            .saturating_add(thread.usage.output_tokens.reported)
                    });
                let units = self
                    .store
                    .list(&self.repository)
                    .map_err(issue_error)?
                    .iter()
                    .filter(|item| item.batch_id == assignment.batch_id)
                    .count()
                    .max(1) as u64;
                if usage >= assignment.execution_budget() / units {
                    self.stop(&assignment)?;
                    let latest = self.store.read(&id).map_err(issue_error)?;
                    self.store
                        .apply(
                            &format!("{id}-budget-{}", latest.revision),
                            &id,
                            latest.revision,
                            IssueAssignmentCommand::ExecutionFailed {
                                detail:
                                    "Issue execution reached its share of the batch token budget"
                                        .into(),
                            },
                            now()?,
                        )
                        .map_err(issue_error)?;
                    continue;
                }
            }
            let now = now()?;
            if assignment
                .lease_until
                .is_some_and(|deadline| deadline < now.saturating_add(75) && deadline > now)
            {
                if let Err(error) = self.store.apply(
                    &format!("{id}-heartbeat-{}", assignment.revision),
                    &id,
                    assignment.revision,
                    IssueAssignmentCommand::Renew {
                        epoch,
                        lease_until: now.saturating_add(120),
                    },
                    now,
                ) {
                    log::debug!("Issue heartbeat deferred: {error}");
                }
            }
        }
        Ok(())
    }

    fn auto_claim(&self) -> Result<(), RpcError> {
        {
            let mut last = self
                .auto_checked
                .lock()
                .map_err(|_| issue_error("Auto-claim timer lock poisoned".into()))?;
            if last.is_some_and(|time| time.elapsed() < Duration::from_secs(60)) {
                return Ok(());
            }
            *last = Some(std::time::Instant::now());
        }
        let config = self
            .changes
            .config
            .read_snapshot()
            .map_err(|error| issue_error(error.to_string()))?;
        let Some(workflow) = config.values.issues.repositories.get(&self.repository) else {
            return Ok(());
        };
        let Some(rule) = &workflow.auto_claim else {
            return Ok(());
        };
        let Some(activation) = self
            .store
            .auto_claim_activation(&self.repository)
            .map_err(issue_error)?
        else {
            return Ok(());
        };
        let prefix = format!("issue-auto-{activation}-");
        let existing = self.store.list(&self.repository).map_err(issue_error)?;
        let handled = existing
            .iter()
            .filter(|assignment| assignment.batch_id.starts_with(&prefix))
            .flat_map(|assignment| {
                assignment
                    .item
                    .issues
                    .iter()
                    .map(|issue| issue.node_id.clone())
            })
            .collect::<BTreeSet<_>>();
        let remaining = (rule.max_issues as usize).saturating_sub(handled.len());
        if remaining == 0 {
            return Ok(());
        }
        let repository = self
            .changes
            .worktree_runtime
            .block_on(super::issue_operations::repository(&self.changes.dir_root))
            .map_err(issue_error)?;
        let github = zeta_github::GitHub::default();
        let info = self
            .changes
            .worktree_runtime
            .block_on(super::issue_assignment_operations::sync_call(
                github.issue_repository(&repository),
                &self.sync_cancel.token(),
            ))
            .map_err(issue_error)?;
        let identity = zeta_work_coordination::IssueRepositoryIdentity {
            host: repository.host.clone(),
            node_id: info.node_id,
            owner: repository.owner.clone(),
            name: repository.name.clone(),
        };
        if identity.key() != self.repository {
            return Err(issue_error(
                "Automatic claiming stopped because the repository identity changed".into(),
            ));
        }
        let candidates = self
            .changes
            .worktree_runtime
            .block_on(super::issue_assignment_operations::sync_call(
                github.automatic_issue_candidates(
                    &repository,
                    &rule.labels,
                    rule.assignee.as_deref(),
                    remaining + handled.len(),
                ),
                &self.sync_cancel.token(),
            ))
            .map_err(issue_error)?;
        let mut items = Vec::new();
        for candidate in candidates {
            if handled.contains(&candidate.node_id)
                || existing.iter().any(|assignment| {
                    !matches!(
                        assignment.ownership,
                        IssueOwnership::Unclaimed
                            | IssueOwnership::Released
                            | IssueOwnership::Completed
                    ) && assignment
                        .item
                        .issues
                        .iter()
                        .any(|issue| issue.node_id == candidate.node_id)
                })
            {
                continue;
            }
            if candidate.issue.labels.iter().any(|label| {
                workflow.labels.all().contains(&label.name.as_str())
                    && label.name != workflow.labels.todo
            }) {
                continue;
            }
            let snapshot = self
                .changes
                .worktree_runtime
                .block_on(super::issue_assignment_operations::sync_call(
                    github.issue(&repository, candidate.issue.number),
                    &self.sync_cancel.token(),
                ))
                .map_err(issue_error)?;
            let material_digest = super::issue_assignment::issue_material_digest(&snapshot)?;
            items.push(zeta_work_coordination::IssueWorkItem {
                id: format!("issue-{}", candidate.issue.number),
                issues: vec![zeta_work_coordination::IssueIdentity {
                    node_id: candidate.node_id,
                    number: candidate.issue.number,
                    title: candidate.issue.title.clone(),
                    updated_at: candidate.issue.updated_at,
                    material_digest,
                }],
                objective: candidate.issue.title,
                acceptance_conditions: vec![
                    "Implement the issue requirements and pass the configured validation commands"
                        .into(),
                ],
                scope: Default::default(),
                dependencies: BTreeSet::new(),
                agent: workflow.worker_agent.clone(),
            });
            if items.len() == remaining {
                break;
            }
        }
        if items.is_empty() {
            return Ok(());
        }
        let base = workflow
            .base_branch
            .as_deref()
            .unwrap_or(&info.default_branch);
        let commit = self
            .changes
            .worktree_runtime
            .block_on(async {
                let git = zeta_git::GitClient::system();
                let source = git.open_repository(&self.changes.dir_root).await?;
                git.fetch_branch(&source, base).await
            })
            .map_err(|error| issue_error(error.to_string()))?;
        let batch = format!(
            "{prefix}{}",
            ContentDigest::sha256(
                &serde_json::to_vec(&items).map_err(|error| issue_error(error.to_string()))?
            )
            .to_string()
            .replace(':', "-")
        );
        let plan = IssueAssignmentPlan {
            repository: identity,
            workflow: workflow.clone(),
            config_revision: config.revision.get(),
            model: config.values.preferred_model.clone(),
            planning_tokens: 0,
            base_commit: commit,
            target_branch: workflow
                .target_branch
                .clone()
                .unwrap_or(info.default_branch),
            items,
        };
        self.store
            .claim(&batch, &plan, now()?)
            .map_err(issue_error)?;
        Ok(())
    }

    fn fail(&self, id: &str, detail: &str) -> Result<(), RpcError> {
        let current = self.store.read(id).map_err(issue_error)?;
        self.stop(&current)?;
        let latest = self.store.read(id).map_err(issue_error)?;
        self.store
            .apply(
                &format!("{id}-failed-{}", latest.revision),
                id,
                latest.revision,
                IssueAssignmentCommand::ExecutionFailed {
                    detail: detail.into(),
                },
                now()?,
            )
            .map_err(issue_error)?;
        Ok(())
    }

    fn block(&self, id: &str, detail: &str) -> Result<(), RpcError> {
        let current = self.store.read(id).map_err(issue_error)?;
        if current.ownership != IssueOwnership::Held {
            return Ok(());
        }
        self.stop(&current)?;
        let latest = self.store.read(id).map_err(issue_error)?;
        self.store
            .apply(
                &format!("{id}-blocked-{}", latest.revision),
                id,
                latest.revision,
                IssueAssignmentCommand::SyncFailed {
                    state: IssueSyncState::Conflict,
                    detail: detail.into(),
                },
                now()?,
            )
            .map_err(issue_error)?;
        Ok(())
    }

    fn audit_remote(&self) -> Result<(), RpcError> {
        for assignment in self.store.list(&self.repository).map_err(issue_error)? {
            if self.stop.load(Ordering::Acquire) {
                break;
            }
            if assignment.ownership != IssueOwnership::Held
                || assignment.sync_state == IssueSyncState::Conflict
                || assignment.sync_state != IssueSyncState::Synced
                    && !assignment
                        .delivery
                        .as_ref()
                        .is_some_and(|receipt| receipt.pull_request_number.is_some())
            {
                continue;
            }
            let timer = format!("audit:{}", assignment.id);
            if self
                .next_sync
                .lock()
                .map_err(|_| issue_error("Issue audit timer lock poisoned".into()))?
                .get(&timer)
                .is_some_and(|deadline| std::time::Instant::now() < *deadline)
            {
                continue;
            }
            self.next_sync
                .lock()
                .map_err(|_| issue_error("Issue audit timer lock poisoned".into()))?
                .insert(
                    timer,
                    std::time::Instant::now()
                        + Duration::from_secs(
                            if assignment
                                .delivery
                                .as_ref()
                                .is_some_and(|receipt| receipt.pull_request_number.is_some())
                            {
                                5
                            } else {
                                60
                            },
                        ),
                );
            match super::issue_delivery::audit_issue_remote(
                &self.changes,
                &assignment,
                &self.sync_cancel.token(),
            ) {
                Ok(super::issue_delivery::RemoteIssueState::Unchanged) => {}
                Ok(super::issue_delivery::RemoteIssueState::Merged) => {
                    super::issue_delivery::finish_issue_delivery(
                        &self.store,
                        &self.changes,
                        &assignment,
                        &self.sync_cancel.token(),
                    )?;
                }
                Ok(super::issue_delivery::RemoteIssueState::Conflict(reason)) => {
                    let _gate = self
                        .gate
                        .lock()
                        .map_err(|_| issue_error("Issue scheduler lock poisoned".into()))?;
                    let current = self.store.read(&assignment.id).map_err(issue_error)?;
                    if current.epoch == assignment.epoch {
                        self.block(&current.id, &reason)?;
                    }
                }
                Err(error) => {
                    let latest = self.store.read(&assignment.id).map_err(issue_error)?;
                    if latest.epoch == assignment.epoch
                        && latest.sync_state == IssueSyncState::Synced
                    {
                        self.store
                            .apply(
                                &format!("{}-unavailable-{}", latest.id, latest.revision),
                                &latest.id,
                                latest.revision,
                                IssueAssignmentCommand::SyncFailed {
                                    state: IssueSyncState::Pending,
                                    detail: format!("GitHub status unavailable: {error}"),
                                },
                                now()?,
                            )
                            .map_err(issue_error)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn sync_pending(&self) -> Result<(), RpcError> {
        for assignment in self.store.list(&self.repository).map_err(issue_error)? {
            if self.stop.load(Ordering::Acquire) {
                break;
            }
            if assignment.sync_state != IssueSyncState::Pending
                || matches!(
                    assignment.ownership,
                    IssueOwnership::Unclaimed
                        | IssueOwnership::Completed
                        | IssueOwnership::Released
                )
            {
                continue;
            }
            let due = self
                .next_sync
                .lock()
                .map_err(|_| issue_error("Issue sync timer lock poisoned".into()))?
                .get(&assignment.id)
                .is_none_or(|deadline| std::time::Instant::now() >= *deadline);
            if !due {
                continue;
            }
            match super::issue_assignment_operations::sync_assignment(
                &self.store,
                &self.changes,
                &assignment,
                assignment.desired_stage,
                &self.sync_cancel.token(),
            ) {
                Ok(_) => {
                    self.next_sync
                        .lock()
                        .map_err(|_| issue_error("Issue sync timer lock poisoned".into()))?
                        .remove(&assignment.id);
                }
                Err(error) => {
                    self.next_sync
                        .lock()
                        .map_err(|_| issue_error("Issue sync timer lock poisoned".into()))?
                        .insert(
                            assignment.id.clone(),
                            std::time::Instant::now() + Duration::from_secs(60),
                        );
                    let latest = self.store.read(&assignment.id).map_err(issue_error)?;
                    if latest.epoch == assignment.epoch
                        && latest.sync_state != IssueSyncState::Conflict
                    {
                        let _ = self.store.apply(
                            &format!("{}-sync-error-{}", latest.id, latest.revision),
                            &latest.id,
                            latest.revision,
                            IssueAssignmentCommand::SyncFailed {
                                state: IssueSyncState::Pending,
                                detail: error.to_string(),
                            },
                            now()?,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn notify_changes(&self) -> Result<(), RpcError> {
        let mut notices = self
            .notices
            .lock()
            .map_err(|_| issue_error("Issue notification lock poisoned".into()))?;
        for assignment in self.store.list(&self.repository).map_err(issue_error)? {
            let disconnected = assignment
                .lease_until
                .is_some_and(|deadline| now().is_ok_and(|time| time >= deadline));
            let state = format!(
                "{:?}/{:?}/{:?}/{disconnected}/{}",
                assignment.ownership,
                assignment.desired_stage,
                assignment.sync_state == IssueSyncState::Conflict,
                assignment.paused
            );
            let changed = notices
                .insert(assignment.id.clone(), state.clone())
                .is_some_and(|old| old != state);
            if changed
                && (disconnected
                    || assignment.sync_state == IssueSyncState::Conflict
                    || matches!(
                        assignment.desired_stage,
                        zeta_work_coordination::IssueStage::Review
                            | zeta_work_coordination::IssueStage::Blocked
                            | zeta_work_coordination::IssueStage::Completed
                    ))
            {
                let message = format!(
                    "Issue {}: {}. {} Open /issue → assignments.",
                    assignment
                        .item
                        .issues
                        .iter()
                        .map(|issue| format!("#{}", issue.number))
                        .collect::<Vec<_>>()
                        .join(", "),
                    if disconnected {
                        "executor disconnected"
                    } else if assignment.sync_state == IssueSyncState::Conflict {
                        "needs a decision"
                    } else if assignment.desired_stage
                        == zeta_work_coordination::IssueStage::Completed
                    {
                        "delivered"
                    } else if assignment.desired_stage == zeta_work_coordination::IssueStage::Review
                    {
                        "ready for verification"
                    } else {
                        "paused or blocked"
                    },
                    assignment.detail
                );
                self.updates.publish_issue_notice(
                    zeta_app_server_protocol::protocol::issue_assignment::IssueAssignmentNotice {
                        assignment_id: assignment.id,
                        message,
                    },
                );
            }
        }
        Ok(())
    }

    fn tick(&self) -> Result<(), RpcError> {
        let _guard = self
            .gate
            .lock()
            .map_err(|_| issue_error("Issue scheduler lock poisoned".into()))?;
        let assignments = self.store.list(&self.repository).map_err(issue_error)?;
        let mut active = BTreeMap::<String, u32>::new();
        for assignment in &assignments {
            if let Some(thread) = &assignment.thread_id {
                let task = self
                    .tasks
                    .read_session(thread.as_str())
                    .map_err(issue_error)?
                    .ok_or_else(|| {
                        issue_error("Issue assignment lost its source directory binding".into())
                    })?;
                if task.source_root != self.changes.dir_root {
                    continue;
                }
            }
            if assignment.ownership != IssueOwnership::Held {
                continue;
            }
            let owned_epoch = {
                self.owned
                    .lock()
                    .map_err(|_| issue_error("Issue execution ownership lock poisoned".into()))?
                    .get(&assignment.id)
                    .copied()
            };
            if let Some(epoch) = owned_epoch {
                if epoch != assignment.epoch || !assignment.auto_start {
                    continue;
                }
                if assignment
                    .lease_until
                    .is_some_and(|deadline| now().is_ok_and(|time| time >= deadline))
                {
                    self.fail(
                        &assignment.id,
                        "Execution lease expired; review before resuming",
                    )?;
                    continue;
                }
                if assignment.sync_state == IssueSyncState::Conflict {
                    self.block(&assignment.id, &assignment.detail)?;
                    continue;
                }
                let thread = assignment
                    .thread_id
                    .as_ref()
                    .ok_or_else(|| issue_error("Running Issue omitted its Thread".into()))?;
                let snapshot = self
                    .changes
                    .threads
                    .read_thread(thread)
                    .map_err(super::core_error)?;
                if let Some(turn) = snapshot.turns.last() {
                    if turn.status == zeta_protocol::TurnStatus::Completed {
                        if snapshot.goal.as_ref().is_some_and(|goal| {
                            goal.status == zeta_protocol::ThreadGoalStatus::Active
                        }) {
                            *active.entry(assignment.batch_id.clone()).or_default() += 1;
                            continue;
                        }
                        let records = self
                            .changes
                            .store
                            .list_for_thread(thread)
                            .map_err(|error| issue_error(error.to_string()))?
                            .into_iter()
                            .filter(|record| {
                                record.work_attempt.as_ref().is_some_and(|identity| {
                                    Some(&identity.attempt_id) == assignment.attempt_id.as_ref()
                                })
                            })
                            .collect::<Vec<_>>();
                        let failure = self
                            .changes
                            .capture_failures
                            .read()
                            .map_err(|_| issue_error("Capture failure lock poisoned".into()))?;
                        let failed = records
                            .iter()
                            .find_map(|record| failure.get(&record.turn_id).cloned());
                        drop(failure);
                        if let Some(failure) = failed {
                            self.fail(
                                &assignment.id,
                                &format!("Result capture failed: {failure:?}"),
                            )?;
                            continue;
                        }
                        if records.is_empty()
                            || records.iter().any(|record| {
                                record.capture_state != zeta_turn_changes::CaptureState::Sealed
                            })
                        {
                            *active.entry(assignment.batch_id.clone()).or_default() += 1;
                            continue;
                        }
                        if let Err(error) = self.seal(assignment) {
                            self.fail(&assignment.id, &format!("Result capture failed: {error}"))?;
                        }
                        continue;
                    }
                    if matches!(
                        turn.status,
                        zeta_protocol::TurnStatus::Failed | zeta_protocol::TurnStatus::Interrupted
                    ) {
                        self.stop(assignment)?;
                        continue;
                    }
                }
                *active.entry(assignment.batch_id.clone()).or_default() += 1;
            }
        }
        for assignment in assignments {
            if let Some(thread) = &assignment.thread_id {
                let task = self
                    .tasks
                    .read_session(thread.as_str())
                    .map_err(issue_error)?
                    .ok_or_else(|| {
                        issue_error("Issue assignment lost its source directory binding".into())
                    })?;
                if task.source_root != self.changes.dir_root {
                    continue;
                }
            }
            if assignment.ownership != IssueOwnership::Held
                || !assignment.auto_start
                || assignment.lease_until.is_some()
                || assignment.sync_state != IssueSyncState::Synced
                || assignment.work_run_id.is_none()
            {
                continue;
            }
            if *active.get(&assignment.batch_id).unwrap_or(&0) >= assignment.workflow.max_parallel {
                continue;
            }
            let run = self
                .work
                .read(assignment.work_run_id.as_ref().expect("checked run"))
                .map_err(|error| issue_error(error.to_string()))?;
            let siblings = self.store.list(&self.repository).map_err(issue_error)?;
            if siblings.iter().any(|sibling| {
                sibling.id != assignment.id
                    && sibling.lease_until.is_some()
                    && assignment.item.conflicts_with(&sibling.item)
            }) {
                continue;
            }
            if assignment.item.dependencies.iter().any(|dependency| {
                !siblings.iter().any(|sibling| {
                    sibling.batch_id == assignment.batch_id
                        && &sibling.item.id == dependency
                        && sibling.ownership == IssueOwnership::Completed
                })
            }) {
                continue;
            }
            let current = self.store.read(&assignment.id).map_err(issue_error)?;
            if current.epoch != assignment.epoch || current.lease_until.is_some() {
                continue;
            }
            let assignment = match self.store.apply(
                &format!("{}-acquire-{}", current.id, current.revision),
                &current.id,
                current.revision,
                IssueAssignmentCommand::Acquire {
                    epoch: current.epoch,
                    lease_until: now()?.saturating_add(120),
                },
                now()?,
            ) {
                Ok(assignment) => assignment,
                Err(error) => {
                    log::debug!("Issue execution admission changed: {error}");
                    continue;
                }
            };
            self.owned
                .lock()
                .map_err(|_| issue_error("Issue execution ownership lock poisoned".into()))?
                .insert(assignment.id.clone(), assignment.epoch);
            match self.begin(&assignment, &run) {
                Ok(()) => *active.entry(assignment.batch_id.clone()).or_default() += 1,
                Err(error) => {
                    self.fail(
                        &assignment.id,
                        &format!("Execution could not start: {error}"),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn begin(&self, assignment: &IssueAssignment, run: &WorkRun) -> Result<(), RpcError> {
        let thread_id = assignment
            .thread_id
            .as_ref()
            .ok_or_else(|| issue_error("Issue has no prepared Thread".into()))?;
        let source = zeta_file_access::Dir::open_local(&self.changes.dir_root)
            .map_err(|error| issue_error(error.to_string()))?;
        let git = zeta_git::GitClient::system();
        let (repository_id, tree, base_commit) = self
            .changes
            .worktree_runtime
            .block_on(async {
                let repository = git
                    .open_repository(&self.changes.dir_root)
                    .await
                    .map_err(|error| error.to_string())?;
                let id = zeta_file_access::Dir::open_local(repository.common_dir())
                    .map_err(|error| error.to_string())?
                    .id();
                let base_commit = git
                    .resolve_commit(
                        &repository,
                        &format!("refs/heads/{}", batch_branch(&run.work_run_id)),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                let tree = git
                    .resolve_tree(&repository, &base_commit)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>((format!("git:{id}"), tree.as_str().to_owned(), base_commit))
            })
            .map_err(issue_error)?;
        let checkpoint = zeta_work_coordination::RootCheckpoint {
            environment_id: source.env().clone(),
            dir_id: source.id(),
            state: zeta_work_coordination::RootState::Git {
                repositories: vec![zeta_work_coordination::GitRepositoryCheckpoint {
                    repository_id,
                    relative_path: ".".into(),
                    target: zeta_work_coordination::GitRootTarget::Branch {
                        name: batch_branch(&run.work_run_id),
                        expected_head: base_commit.clone(),
                    },
                    baseline_tree: tree,
                }],
            },
            control_resources: Vec::new(),
        };
        let executor = self
            .environment
            .read()
            .map_err(|_| issue_error("Environment lock poisoned".into()))?
            .turn_executor
            .clone();
        let role = assignment.agent_role.as_ref().ok_or_else(|| {
            issue_error("Issue Agent definition was not frozen during preparation".into())
        })?;
        let profile = executor
            .tool_profile_for_names(&assignment.agent_tools)
            .map_err(super::core_error)?;
        let role_instructions = zeta_protocol::TurnInstructions::new(
            "issue-workflow",
            "worker",
            "1",
            format!(
                "{}\n{}",
                zeta_models_manager::BASE_INSTRUCTIONS.freeze().body(),
                role.instructions
            ),
        )
        .map_err(|error| issue_error(error.to_string()))?;
        let policy = executor.policy_revision();
        let contract_id =
            WorkContractId::new(format!("{}-contract-{}", assignment.id, assignment.epoch))
                .map_err(|error| issue_error(error.to_string()))?;
        let attempt_id = WorkAttemptId::new(format!("{}-a{}", assignment.id, assignment.epoch))
            .map_err(|error| issue_error(error.to_string()))?;
        let mut current = run.clone();
        if !current.contracts.contains_key(&contract_id) {
            current = apply(
                &self.work,
                &run.work_run_id,
                current.revision,
                &format!("{contract_id}-create"),
                WorkRunCommand::CreateContract {
                    contract: WorkContractDraft {
                        contract_id: contract_id.clone(),
                        goal_revision: run
                            .current_goal()
                            .ok_or_else(|| issue_error("Work run has no goal".into()))?
                            .revision,
                        topology_revision: run.topology_revision,
                        owner_thread_id: thread_id.clone(),
                        objective: assignment.item.objective.clone(),
                        acceptance_conditions: assignment.item.acceptance_conditions.clone(),
                        exclusions: Vec::new(),
                        environment_id: source.env().clone(),
                        roots: vec![checkpoint],
                        primary_root_dir_id: source.id(),
                        authorization: zeta_work_coordination::AuthorizationSnapshotRef {
                            authority: "issue-coordinator".into(),
                            policy_revision: policy.clone(),
                            grant_set_digest: ContentDigest::sha256(
                                serde_json::to_vec(&(thread_id, source.id(), &policy))
                                    .map_err(|error| issue_error(error.to_string()))?
                                    .as_slice(),
                            ),
                            granted_effects_digest: ContentDigest::sha256(
                                serde_json::to_vec(&profile)
                                    .map_err(|error| issue_error(error.to_string()))?
                                    .as_slice(),
                            ),
                        },
                        decision_ids: BTreeSet::new(),
                        upstream_results: run.attempts.values().filter(|attempt| attempt.integration_status == zeta_work_coordination::WorkAttemptIntegrationStatus::Integrated).filter_map(|attempt| attempt.result.as_ref().map(|result| zeta_work_coordination::WorkResultRef { attempt_id: attempt.attempt_id.clone(), result_digest: result.result_digest.clone() })).collect(),
                        expected_scope: assignment.item.scope.clone(),
                        validation_profile: zeta_work_coordination::ValidationProfileRef {
                            name: "issue-command-verifier-v1".into(),
                            content_digest: ContentDigest::sha256(
                                serde_json::to_vec(&assignment.workflow.validation_commands).map_err(|error| issue_error(error.to_string()))?.as_slice(),
                            ),
                        },
                    },
                },
            )?;
        }
        if !current.attempts.contains_key(&attempt_id) {
            current = apply(
                &self.work,
                &run.work_run_id,
                current.revision,
                &format!("{attempt_id}-create"),
                WorkRunCommand::CreateAttempt {
                    attempt_id: attempt_id.clone(),
                    contract: WorkContractRef {
                        contract_id,
                        revision: 1,
                    },
                    participant_thread_id: thread_id.clone(),
                },
            )?;
        }
        if current.attempts[&attempt_id].execution_status == WorkAttemptExecutionStatus::Planned {
            apply(
                &self.work,
                &run.work_run_id,
                current.revision,
                &format!("{attempt_id}-begin"),
                WorkRunCommand::BeginAttempt {
                    attempt_id: attempt_id.clone(),
                    execution_id: WorkExecutionId::new(format!("{attempt_id}-execution"))
                        .map_err(|error| issue_error(error.to_string()))?,
                    mode: zeta_work_coordination::WorkStartMode::Write,
                },
            )?;
        }
        let mut latest = self.store.read(&assignment.id).map_err(issue_error)?;
        if latest.attempt_id.as_ref() != Some(&attempt_id) {
            latest = self
                .store
                .apply(
                    &format!("{attempt_id}-bind"),
                    &latest.id,
                    latest.revision,
                    IssueAssignmentCommand::RecordWork {
                        thread_id: thread_id.clone(),
                        work_run_id: run.work_run_id.clone(),
                        attempt_id: attempt_id.clone(),
                    },
                    now()?,
                )
                .map_err(issue_error)?;
        }
        let snapshot = self
            .changes
            .threads
            .read_thread(thread_id)
            .map_err(super::core_error)?;
        if snapshot
            .goal
            .as_ref()
            .is_none_or(|goal| goal.status == zeta_protocol::ThreadGoalStatus::Complete)
        {
            self.changes
                .threads
                .create_goal(
                    thread_id,
                    assignment.item.objective.clone(),
                    Some({
                        let units = self
                            .store
                            .list(&self.repository)
                            .map_err(issue_error)?
                            .iter()
                            .filter(|item| item.batch_id == assignment.batch_id)
                            .count()
                            .max(1) as u64;
                        assignment.execution_budget()
                            / units
                            / (u64::from(assignment.workflow.max_parallel) / units).max(1)
                    }),
                )
                .map_err(super::core_error)?;
        }
        let snapshot = self
            .changes
            .threads
            .read_thread(thread_id)
            .map_err(super::core_error)?;
        let materials = self
            .tasks
            .read_session(thread_id.as_str())
            .map_err(issue_error)?
            .ok_or_else(|| issue_error("Issue materials unavailable".into()))?;
        let mut input = vec![zeta_protocol::UserInput::Text {
            text: format!(
                "{}\n\nAcceptance conditions:\n{}",
                assignment.item.objective,
                assignment
                    .item
                    .acceptance_conditions
                    .iter()
                    .map(|condition| format!("- {condition}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        }];
        for issue in materials.issues {
            input.push(zeta_protocol::UserInput::Context {
                name: format!("GitHub issue #{}", issue.issue.number),
                content: serde_json::to_string(&issue)
                    .map_err(|error| issue_error(error.to_string()))?,
            });
        }
        let turn = self
            .changes
            .threads
            .start_turn(
                thread_id,
                StartTurnRequest {
                    command_id: command(&format!("{attempt_id}-turn"))?,
                    expected_sequence: SequenceExpectation::Exact(snapshot.sequence),
                    model: role.model.clone(),
                    kind: zeta_protocol::TurnKind::Coding,
                    instructions: role_instructions,
                    policy_revision: policy,
                    approval_mode: zeta_protocol::ApprovalMode::default(),
                    tool_mode: zeta_protocol::ToolMode::Direct,
                    tool_profile: Some(profile),
                    activated_skills: Vec::new(),
                    input,
                },
            )
            .map_err(super::core_error)?;
        latest = self
            .store
            .apply(
                &format!("{attempt_id}-lease-{}", latest.revision),
                &latest.id,
                latest.revision,
                IssueAssignmentCommand::Renew {
                    epoch: latest.epoch,
                    lease_until: now()?.saturating_add(120),
                },
                now()?,
            )
            .map_err(issue_error)?;
        self.owned
            .lock()
            .map_err(|_| issue_error("Issue execution ownership lock poisoned".into()))?
            .insert(latest.id, latest.epoch);
        self.backend
            .start(thread_id, &turn.turn_id)
            .map_err(super::core_error)?;
        let current = self.store.read(&assignment.id).map_err(issue_error)?;
        self.store
            .apply(
                &format!("{}-running-{}", current.id, current.revision),
                &current.id,
                current.revision,
                IssueAssignmentCommand::PrepareSync {
                    stage: zeta_work_coordination::IssueStage::InProgress,
                },
                now()?,
            )
            .map_err(issue_error)?;
        Ok(())
    }

    fn seal(&self, assignment: &IssueAssignment) -> Result<(), RpcError> {
        if let Some(thread) = &assignment.thread_id {
            self.agents
                .cancel_descendants(thread)
                .map_err(super::core_error)?;
        }
        let run_id = assignment
            .work_run_id
            .as_ref()
            .ok_or_else(|| issue_error("Issue has no work run".into()))?;
        let attempt_id = assignment
            .attempt_id
            .as_ref()
            .ok_or_else(|| issue_error("Issue has no attempt".into()))?;
        let run = self
            .work
            .read(run_id)
            .map_err(|error| issue_error(error.to_string()))?;
        if run.attempts[attempt_id].execution_status != WorkAttemptExecutionStatus::Sealed {
            let evidence = self
                .changes
                .derive_attempt_result(&run, attempt_id)
                .map_err(issue_error)?;
            apply(
                &self.work,
                run_id,
                run.revision,
                &format!("{attempt_id}-seal"),
                WorkRunCommand::SealAttempt {
                    attempt_id: attempt_id.clone(),
                    result_digest: evidence.result_digest,
                    change_set_ids: evidence.change_set_ids,
                    private_output_digest: evidence.private_output_digest,
                    external_effects_digest: evidence.external_effects_digest,
                    external_effects_status: evidence.external_effects_status,
                },
            )?;
        }
        self.owned
            .lock()
            .map_err(|_| issue_error("Issue execution ownership lock poisoned".into()))?
            .remove(&assignment.id);
        let current = self.store.read(&assignment.id).map_err(issue_error)?;
        if current.epoch != assignment.epoch {
            return Err(issue_error(
                "Issue execution was superseded during result capture".into(),
            ));
        }
        self.store
            .apply(
                &format!("{attempt_id}-stop-lease"),
                &assignment.id,
                current.revision,
                IssueAssignmentCommand::FinishExecution {
                    epoch: assignment.epoch,
                },
                now()?,
            )
            .map_err(issue_error)?;
        Ok(())
    }
}

fn command(value: &str) -> Result<CommandId, RpcError> {
    CommandId::new(value).map_err(|error| issue_error(error.to_string()))
}
fn apply(
    runtime: &WorkCoordinationRuntime,
    run_id: &WorkRunId,
    revision: u64,
    id: &str,
    command_value: WorkRunCommand,
) -> Result<WorkRun, RpcError> {
    runtime
        .apply(WorkRunCommandRequest {
            command_id: command(id)?,
            work_run_id: run_id.clone(),
            expected_revision: revision,
            command: command_value,
        })
        .map_err(|error| issue_error(error.to_string()))?;
    runtime
        .read(run_id)
        .map_err(|error| issue_error(error.to_string()))
}

pub(super) fn batch_branch(run_id: &WorkRunId) -> String {
    format!(
        "codex/issue-batch-{}",
        &ContentDigest::sha256(run_id.as_str().as_bytes())
            .to_string()
            .replace(':', "-")[..24]
    )
}
