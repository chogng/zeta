use super::multi_agent_evaluation::EvaluationRootAgent;
use super::multi_agent_evaluation::MultiAgentEvaluationHost;
use super::multi_agent_evaluation::MultiSessionEvaluationAgentAttempt;
use super::multi_agent_evaluation::TeamEvaluationAttemptRequest;
use super::multi_agent_evaluation::apply;
use super::multi_agent_evaluation::authorization_snapshot;
use super::multi_agent_evaluation::capture_checkpoint;
use super::multi_agent_evaluation::create_root_agent;
use super::multi_agent_evaluation::evaluation_instructions;
use super::multi_agent_evaluation::evaluation_validation_profile;
use super::multi_agent_evaluation::primary_managed_root;
use super::multi_agent_evaluation::validate_request;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use zeta_core::SequenceExpectation;
use zeta_core::SpawnAgentRequest;
use zeta_core::StartTurnRequest;
use zeta_core::TurnExecutionBackend;
use zeta_file_access::Dir;
use zeta_protocol::AgentContextMode;
use zeta_protocol::AgentRoleSnapshot;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::DelegatedCapabilityScope;
use zeta_protocol::DelegatedPolicyCeiling;
use zeta_protocol::DelegatedTask;
use zeta_protocol::DelegationId;
use zeta_protocol::ModelRef;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ToolMode;
use zeta_protocol::TurnId;
use zeta_protocol::TurnKind;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;
use zeta_work_coordination::WorkContractDraft;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRelationKind;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkStartMode;
use zeta_work_coordination::WorkVerificationStatus;
use zeta_work_coordination::WorkWaitCondition;

/// One root or delegated Agent bound to an exact writable WorkAttempt.
#[derive(Clone, Debug)]
pub struct DevelopmentEvaluationAttempt {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub turn_id: TurnId,
    pub attempt_id: WorkAttemptId,
    pub execution_id: WorkExecutionId,
    pub managed_root: PathBuf,
}

/// One complete single-Agent evaluation setup before its Turn is dispatched.
#[derive(Clone, Debug)]
pub struct SingleAgentDevelopmentEvaluation {
    pub work_run_id: WorkRunId,
    pub agent: DevelopmentEvaluationAttempt,
    pub model: Option<ModelRef>,
    pub source_root: PathBuf,
}

/// Team root whose model loop must consume exact pre-admitted child requests through Agent tools.
#[derive(Clone, Debug)]
pub struct TeamLoopEvaluationCoordinator {
    pub work_run_id: WorkRunId,
    pub root_session_id: SessionId,
    pub root_thread_id: ThreadId,
    pub root_turn_id: TurnId,
    pub model: Option<ModelRef>,
    pub source_root: PathBuf,
    run_key: String,
    request: TeamEvaluationAttemptRequest,
    checkpoint: zeta_work_coordination::RootCheckpoint,
}

/// Child identity, task, and contract boundary admitted before its first Turn is dispatched.
#[derive(Clone, Debug)]
pub struct TeamLoopChildRequest {
    pub delegation_id: DelegationId,
    pub title: String,
    pub task: String,
    pub expected_scope: WorkScopeClaim,
}

/// Exact Team root and writable children after admission binds the reserved tree to WorkRun.
#[derive(Clone, Debug)]
pub struct TeamLoopDevelopmentEvaluation {
    pub coordinator: TeamLoopEvaluationCoordinator,
    pub children: Vec<DevelopmentEvaluationAttempt>,
}

/// Exact content selected by the independent evaluation acceptance profile.
#[derive(Clone, Debug)]
pub struct EvaluationExpectedFile {
    pub path: String,
    pub content: String,
}

/// Verified WorkRun snapshot and its exact verification identity.
#[derive(Clone, Debug)]
pub struct EvaluationVerification {
    pub verification_key: ContentDigest,
    pub work_run: WorkRun,
}

impl MultiAgentEvaluationHost<'_> {
    /// Creates one root Agent, one WorkRun, and one active WorkAttempt without dispatching the
    /// Agent Turn.
    pub fn create_single_agent_development(
        &self,
        request: TeamEvaluationAttemptRequest,
    ) -> Result<SingleAgentDevelopmentEvaluation, String> {
        validate_request(&request)?;
        let changes = self.changes()?;
        let runtime = self.runtime()?;
        let source = Dir::open_local(&changes.dir_root).map_err(|error| error.to_string())?;
        let checkpoint = capture_checkpoint(&source)?;
        let executor = self.server.turn_executor_snapshot();
        let tool_profile = executor
            .tool_profile_snapshot()
            .map_err(|error| error.to_string())?;
        let policy_revision = executor.policy_revision();
        let model = self
            .server
            .model_catalog
            .configured_default()
            .map_err(|error| error.to_string())?;
        let agent = create_root_agent(
            self.server,
            loop_command(&request.run_key, "single-thread")?,
            loop_command(&request.run_key, "single-turn")?,
            format!("Single-Agent evaluation {}", request.run_key),
            request.task.clone(),
            model.clone(),
            &policy_revision,
            &tool_profile,
        )?;
        let work_run_id = loop_run_id(&request.run_key)?;
        let contract_id = loop_contract_id(&request.run_key, "single")?;
        let attempt_id = loop_attempt_id(&request.run_key, "single")?;
        let execution_id = loop_execution_id(&request.run_key, "single")?;
        let mut run = apply(
            runtime,
            &work_run_id,
            0,
            loop_command(&request.run_key, "create-single-run")?,
            WorkRunCommand::Create {
                objective: request.objective.clone(),
                acceptance_conditions: request.acceptance_conditions.clone(),
                exclusions: request.exclusions.clone(),
                root_participant: root_participant(&agent),
            },
        )?;
        let authorization = authorization_snapshot(
            &agent.session_id,
            &agent.thread_id,
            &source,
            &tool_profile.tool_names,
            &policy_revision,
        )?;
        run = apply(
            runtime,
            &work_run_id,
            run.revision,
            loop_command(&request.run_key, "create-single-contract")?,
            WorkRunCommand::CreateContract {
                contract: contract(
                    contract_id.clone(),
                    &agent.thread_id,
                    &request,
                    run.topology_revision,
                    &source,
                    checkpoint,
                    authorization,
                    request.expected_scope.clone(),
                ),
            },
        )?;
        run = apply(
            runtime,
            &work_run_id,
            run.revision,
            loop_command(&request.run_key, "create-single-attempt")?,
            WorkRunCommand::CreateAttempt {
                attempt_id: attempt_id.clone(),
                contract: WorkContractRef {
                    contract_id,
                    revision: 1,
                },
                participant_thread_id: agent.thread_id.clone(),
            },
        )?;
        apply(
            runtime,
            &work_run_id,
            run.revision,
            loop_command(&request.run_key, "begin-single-attempt")?,
            WorkRunCommand::BeginAttempt {
                attempt_id: attempt_id.clone(),
                execution_id: execution_id.clone(),
                mode: WorkStartMode::Write,
            },
        )?;
        Ok(SingleAgentDevelopmentEvaluation {
            work_run_id,
            agent: DevelopmentEvaluationAttempt {
                session_id: agent.session_id,
                thread_id: agent.thread_id.clone(),
                turn_id: agent.turn_id,
                attempt_id,
                execution_id,
                managed_root: primary_managed_root(changes, &agent.thread_id)?,
            },
            model,
            source_root: source.canonical_path().to_path_buf(),
        })
    }

    /// Creates the Team root's durable setup without dispatching it. The root model must use the
    /// ordinary Agent tools to consume its admitted child requests.
    pub fn create_team_loop_coordinator(
        &self,
        request: TeamEvaluationAttemptRequest,
    ) -> Result<TeamLoopEvaluationCoordinator, String> {
        validate_request(&request)?;
        let changes = self.changes()?;
        let runtime = self.runtime()?;
        let source = Dir::open_local(&changes.dir_root).map_err(|error| error.to_string())?;
        let checkpoint = capture_checkpoint(&source)?;
        let executor = self.server.turn_executor_snapshot();
        let tool_profile = executor
            .tool_profile_snapshot()
            .map_err(|error| error.to_string())?;
        let policy_revision = executor.policy_revision();
        let model = self
            .server
            .model_catalog
            .configured_default()
            .map_err(|error| error.to_string())?;
        let root = create_root_agent(
            self.server,
            loop_command(&request.run_key, "team-root-thread")?,
            loop_command(&request.run_key, "team-root-turn")?,
            format!("Team loop evaluation {}", request.run_key),
            request.task.clone(),
            model.clone(),
            &policy_revision,
            &tool_profile,
        )?;
        let work_run_id = loop_run_id(&request.run_key)?;
        apply(
            runtime,
            &work_run_id,
            0,
            loop_command(&request.run_key, "create-team-loop-run")?,
            WorkRunCommand::Create {
                objective: request.objective.clone(),
                acceptance_conditions: request.acceptance_conditions.clone(),
                exclusions: request.exclusions.clone(),
                root_participant: root_participant(&root),
            },
        )?;
        Ok(TeamLoopEvaluationCoordinator {
            work_run_id,
            root_session_id: root.session_id,
            root_thread_id: root.thread_id,
            root_turn_id: root.turn_id,
            model,
            source_root: source.canonical_path().to_path_buf(),
            run_key: request.run_key.clone(),
            request,
            checkpoint,
        })
    }

    /// Reserves exact child delegations and binds their participants, contracts, isolated roots,
    /// and WorkAttempts before the Team root can dispatch them through `spawn_agent`.
    pub fn bind_team_loop_children(
        &self,
        coordinator: &TeamLoopEvaluationCoordinator,
        child_requests: Vec<TeamLoopChildRequest>,
    ) -> Result<TeamLoopDevelopmentEvaluation, String> {
        if child_requests.is_empty() {
            return Err("Team loop requires at least one delegated child".into());
        }
        let mut seen = BTreeSet::new();
        for request in &child_requests {
            if !seen.insert(request.delegation_id.clone()) {
                return Err("Team loop repeats a delegation identity".into());
            }
            if request.title.trim().is_empty() || request.task.trim().is_empty() {
                return Err("Team loop child title and task are required".into());
            }
        }
        let changes = self.changes()?;
        let runtime = self.runtime()?;
        let source =
            Dir::open_local(&coordinator.source_root).map_err(|error| error.to_string())?;
        let executor = self.server.turn_executor_snapshot();
        let tool_profile = executor
            .tool_profile_snapshot()
            .map_err(|error| error.to_string())?;
        let policy_revision = executor.policy_revision();
        for request in &child_requests {
            self.server
                .multi_agent
                .spawn(SpawnAgentRequest {
                    delegation_id: request.delegation_id.clone(),
                    session_id: coordinator.root_session_id.clone(),
                    parent_thread_id: coordinator.root_thread_id.clone(),
                    parent_turn_id: coordinator.root_turn_id.clone(),
                    task: DelegatedTask {
                        title: request.title.clone(),
                        instructions: request.task.clone(),
                    },
                    role: AgentRoleSnapshot {
                        name: "general".into(),
                        instructions: "Complete the delegated task independently. Return a concise, evidence-backed result to the parent Agent."
                            .into(),
                        model: coordinator.model.clone(),
                        definition: None,
                    },
                    inheritance: AgentContextMode::Fresh,
                    policy_ceiling: DelegatedPolicyCeiling {
                        policy_revision: policy_revision.clone(),
                    },
                    capability_scope: DelegatedCapabilityScope {
                        tools: tool_profile.tool_names.clone(),
                        skills: Vec::new(),
                    },
                })
                .map_err(|error| error.to_string())?;
        }
        let root = self
            .server
            .threads
            .read_thread(&coordinator.root_thread_id)
            .map_err(|error| error.to_string())?;
        if root.session_id != coordinator.root_session_id {
            return Err("Team root Session identity changed".into());
        }
        let mut children = Vec::with_capacity(child_requests.len());
        for request in &child_requests {
            let child_thread_id = root
                .delegations
                .get(&request.delegation_id)
                .and_then(|delegation| delegation.child_thread_id.clone())
                .ok_or_else(|| {
                    format!("delegation {} has no child Thread", request.delegation_id)
                })?;
            let child = self
                .server
                .threads
                .read_thread(&child_thread_id)
                .map_err(|error| error.to_string())?;
            let seed = child
                .agent_context_seed
                .as_ref()
                .ok_or_else(|| "Team child omitted its immutable context seed".to_string())?;
            if child.session_id != coordinator.root_session_id
                || seed.parent_thread_id != coordinator.root_thread_id
                || seed.delegation_id != request.delegation_id
            {
                return Err("Team child does not match the canonical delegated relation".into());
            }
            let turn_id = child
                .turns
                .first()
                .map(|turn| turn.turn_id.clone())
                .ok_or_else(|| "Team child omitted its initial Turn".to_string())?;
            children.push((request.clone(), child_thread_id, turn_id));
        }
        let mut run = runtime
            .read(&coordinator.work_run_id)
            .map_err(|error| error.to_string())?;
        for (request, child_thread_id, _) in &children {
            run = apply(
                runtime,
                &coordinator.work_run_id,
                run.revision,
                loop_command(
                    &coordinator.run_key,
                    &format!("add-child-{}", short_id(request.delegation_id.as_str())),
                )?,
                WorkRunCommand::AddParticipant {
                    participant: WorkParticipant {
                        session_id: coordinator.root_session_id.clone(),
                        thread_id: child_thread_id.clone(),
                        relation: WorkParticipantRelation::Delegated {
                            parent_thread_id: coordinator.root_thread_id.clone(),
                            delegation_id: request.delegation_id.clone(),
                        },
                    },
                },
            )?;
        }
        let topology_revision = run.topology_revision;
        let mut attempts = Vec::with_capacity(children.len());
        for (index, (request, child_thread_id, turn_id)) in children.into_iter().enumerate() {
            let suffix = format!("child-{index}");
            let contract_id = loop_contract_id(&coordinator.run_key, &suffix)?;
            let attempt_id = loop_attempt_id(&coordinator.run_key, &suffix)?;
            let execution_id = loop_execution_id(&coordinator.run_key, &suffix)?;
            let authorization = authorization_snapshot(
                &coordinator.root_session_id,
                &child_thread_id,
                &source,
                &tool_profile.tool_names,
                &policy_revision,
            )?;
            run = apply(
                runtime,
                &coordinator.work_run_id,
                run.revision,
                loop_command(&coordinator.run_key, &format!("create-{suffix}-contract"))?,
                WorkRunCommand::CreateContract {
                    contract: contract(
                        contract_id.clone(),
                        &child_thread_id,
                        &coordinator.request,
                        topology_revision,
                        &source,
                        coordinator.checkpoint.clone(),
                        authorization,
                        request.expected_scope,
                    ),
                },
            )?;
            run = apply(
                runtime,
                &coordinator.work_run_id,
                run.revision,
                loop_command(&coordinator.run_key, &format!("create-{suffix}-attempt"))?,
                WorkRunCommand::CreateAttempt {
                    attempt_id: attempt_id.clone(),
                    contract: WorkContractRef {
                        contract_id,
                        revision: 1,
                    },
                    participant_thread_id: child_thread_id.clone(),
                },
            )?;
            run = apply(
                runtime,
                &coordinator.work_run_id,
                run.revision,
                loop_command(&coordinator.run_key, &format!("begin-{suffix}-attempt"))?,
                WorkRunCommand::BeginAttempt {
                    attempt_id: attempt_id.clone(),
                    execution_id: execution_id.clone(),
                    mode: WorkStartMode::Write,
                },
            )?;
            attempts.push(DevelopmentEvaluationAttempt {
                session_id: coordinator.root_session_id.clone(),
                thread_id: child_thread_id.clone(),
                turn_id,
                attempt_id,
                execution_id,
                managed_root: primary_managed_root(changes, &child_thread_id)?,
            });
        }
        Ok(TeamLoopDevelopmentEvaluation {
            coordinator: coordinator.clone(),
            children: attempts,
        })
    }

    /// Dispatches one exact evaluation Turn through the ordinary executor.
    pub fn start_development_agent(
        &self,
        attempt: &DevelopmentEvaluationAttempt,
    ) -> Result<(), String> {
        self.start_evaluation_turn(&attempt.thread_id, &attempt.turn_id)
    }

    /// Dispatches the Team root through the ordinary executor.
    pub fn start_team_loop_coordinator(
        &self,
        coordinator: &TeamLoopEvaluationCoordinator,
    ) -> Result<(), String> {
        self.start_evaluation_turn(&coordinator.root_thread_id, &coordinator.root_turn_id)
    }

    /// Dispatches one existing cross-Session root Agent through the ordinary executor.
    pub fn start_multi_session_development_agent(
        &self,
        attempt: &MultiSessionEvaluationAgentAttempt,
    ) -> Result<(), String> {
        self.start_evaluation_turn(&attempt.thread_id, &attempt.turn_id)
    }

    /// Resumes one exact cross-Session WorkAttempt with a fresh Turn after its previous Turn was
    /// stopped at the durable wait boundary.
    pub fn resume_multi_session_development_agent(
        &self,
        run_key: &str,
        attempt: &MultiSessionEvaluationAgentAttempt,
        task: String,
    ) -> Result<MultiSessionEvaluationAgentAttempt, String> {
        let thread = self
            .server
            .threads
            .read_thread(&attempt.thread_id)
            .map_err(|error| error.to_string())?;
        if thread.session_id != attempt.session_id
            || thread
                .turns
                .iter()
                .find(|turn| turn.turn_id == attempt.turn_id)
                .is_none_or(|turn| turn.status != TurnStatus::Interrupted)
        {
            return Err(
                "cross-Session wait resume requires the exact interrupted predecessor Turn".into(),
            );
        }
        let executor = self.server.turn_executor_snapshot();
        let tool_profile = executor
            .tool_profile_snapshot()
            .map_err(|error| error.to_string())?;
        let policy_revision = executor.policy_revision();
        let model = self
            .server
            .model_catalog
            .configured_default()
            .map_err(|error| error.to_string())?;
        let turn = self
            .server
            .threads
            .start_turn(
                &attempt.thread_id,
                StartTurnRequest {
                    command_id: loop_command(run_key, "resume-multi-session-turn")?,
                    expected_sequence: SequenceExpectation::Exact(thread.sequence),
                    model,
                    kind: TurnKind::Coding,
                    instructions: evaluation_instructions()?,
                    policy_revision,
                    approval_mode: ApprovalMode::BypassPermissions,
                    tool_mode: ToolMode::Direct,
                    tool_profile: Some(tool_profile),
                    activated_skills: Vec::new(),
                    input: vec![UserInput::Text { text: task }],
                },
            )
            .map_err(|error| error.to_string())?;
        let mut resumed = attempt.clone();
        resumed.turn_id = turn.turn_id;
        self.start_multi_session_development_agent(&resumed)?;
        Ok(resumed)
    }

    /// Places the second independent Session in a durable exact-result wait on the first.
    pub fn create_multi_session_wait(
        &self,
        run_key: &str,
        work_run_id: &WorkRunId,
        waiting: &MultiSessionEvaluationAgentAttempt,
        target: &MultiSessionEvaluationAgentAttempt,
    ) -> Result<WorkRelationId, String> {
        let runtime = self.runtime()?;
        let relation_id = WorkRelationId::new(format!("eval-wait-{run_key}"))
            .map_err(|error| error.to_string())?;
        let run = runtime
            .read(work_run_id)
            .map_err(|error| error.to_string())?;
        apply(
            runtime,
            work_run_id,
            run.revision,
            loop_command(run_key, "create-multi-session-wait")?,
            WorkRunCommand::CreateRelation {
                relation_id: relation_id.clone(),
                source_attempt_id: waiting.attempt_id.clone(),
                target_attempt_id: target.attempt_id.clone(),
                kind: WorkRelationKind::Wait {
                    target_execution_id: target.execution_id.clone(),
                    condition: WorkWaitCondition::AttemptSealed,
                },
            },
        )?;
        Ok(relation_id)
    }

    /// Seals one terminal WorkAttempt using only host-derived ChangeSet, output, and Tool facts.
    pub fn seal_development_attempt(
        &self,
        run_key: &str,
        work_run_id: &WorkRunId,
        attempt_id: &WorkAttemptId,
    ) -> Result<WorkRun, String> {
        let runtime = self.runtime()?;
        let changes = self.changes()?;
        let run = runtime
            .read(work_run_id)
            .map_err(|error| error.to_string())?;
        let result = changes.derive_attempt_result(&run, attempt_id)?;
        apply(
            runtime,
            work_run_id,
            run.revision,
            loop_command(run_key, &format!("seal-{}", short_id(attempt_id.as_str())))?,
            WorkRunCommand::SealAttempt {
                attempt_id: attempt_id.clone(),
                result_digest: result.result_digest,
                change_set_ids: result.change_set_ids,
                private_output_digest: result.private_output_digest,
                external_effects_digest: result.external_effects_digest,
                external_effects_status: result.external_effects_status,
            },
        )
    }

    /// Runs exact-file acceptance on the independently replayed candidate root and records the
    /// resulting verification through the normal WorkRun reducer.
    pub fn verify_development_files(
        &self,
        run_key: &str,
        work_run_id: &WorkRunId,
        attempt_ids: BTreeSet<WorkAttemptId>,
        expected_files: Vec<EvaluationExpectedFile>,
    ) -> Result<EvaluationVerification, String> {
        let expected_files = validate_expected_files(expected_files)?;
        let runtime = self.runtime()?;
        let run = runtime
            .read(work_run_id)
            .map_err(|error| error.to_string())?;
        let (work_run, verification_key) = runtime
            .request_evaluation_verification(
                loop_command(run_key, "begin-exact-file-verification")?,
                loop_command(run_key, "finish-exact-file-verification")?,
                work_run_id.clone(),
                run.revision,
                attempt_ids,
                &expected_files,
            )
            .map_err(|error| error.to_string())?;
        Ok(EvaluationVerification {
            verification_key,
            work_run,
        })
    }

    /// Sends one verified result set through the normal integration request and publication
    /// reconciler.
    pub fn integrate_development_verification(
        &self,
        run_key: &str,
        work_run_id: &WorkRunId,
        verification_key: ContentDigest,
    ) -> Result<WorkRun, String> {
        let runtime = self.runtime()?;
        let run = runtime
            .read(work_run_id)
            .map_err(|error| error.to_string())?;
        if run
            .verifications
            .get(&verification_key)
            .is_none_or(|verification| verification.status != WorkVerificationStatus::Verified)
        {
            return Err("evaluation integration requires the exact verified result set".into());
        }
        runtime
            .request_integration(
                loop_command(run_key, "integrate-exact-file-verification")?,
                work_run_id.clone(),
                run.revision,
                verification_key,
            )
            .map(|result| result.work_run)
            .map_err(|error| error.to_string())
    }

    fn start_evaluation_turn(&self, thread_id: &ThreadId, turn_id: &TurnId) -> Result<(), String> {
        self.server
            .turn_backend
            .start(thread_id, turn_id)
            .map_err(|error| error.to_string())
    }

    fn changes(&self) -> Result<&super::turn_changes_runtime::TurnChangesRuntime, String> {
        self.server
            .turn_changes
            .as_deref()
            .ok_or_else(|| "TurnChanges is unavailable".to_string())
    }

    fn runtime(
        &self,
    ) -> Result<&super::work_coordination_runtime::WorkCoordinationRuntime, String> {
        self.server
            .work_coordination
            .as_deref()
            .ok_or_else(|| "Work coordination is unavailable".to_string())
    }
}

fn root_participant(agent: &EvaluationRootAgent) -> WorkParticipant {
    WorkParticipant {
        session_id: agent.session_id.clone(),
        thread_id: agent.thread_id.clone(),
        relation: WorkParticipantRelation::Root,
    }
}

#[allow(clippy::too_many_arguments)]
fn contract(
    contract_id: WorkContractId,
    owner_thread_id: &ThreadId,
    request: &TeamEvaluationAttemptRequest,
    topology_revision: u64,
    source: &Dir,
    checkpoint: zeta_work_coordination::RootCheckpoint,
    authorization: zeta_work_coordination::AuthorizationSnapshotRef,
    expected_scope: WorkScopeClaim,
) -> WorkContractDraft {
    WorkContractDraft {
        contract_id,
        goal_revision: 1,
        topology_revision,
        owner_thread_id: owner_thread_id.clone(),
        objective: request.objective.clone(),
        acceptance_conditions: request.acceptance_conditions.clone(),
        exclusions: request.exclusions.clone(),
        environment_id: source.env().clone(),
        roots: vec![checkpoint],
        primary_root_dir_id: source.id(),
        authorization,
        decision_ids: BTreeSet::new(),
        upstream_results: Vec::new(),
        expected_scope,
        validation_profile: evaluation_validation_profile(),
    }
}

fn validate_expected_files(
    files: Vec<EvaluationExpectedFile>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if files.is_empty() {
        return Err("evaluation acceptance requires at least one expected file".into());
    }
    let mut validated = BTreeMap::new();
    for file in files {
        let path = Path::new(&file.path);
        if file.path.is_empty()
            || path.is_absolute()
            || !path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err("evaluation expected file path must be a normalized relative path".into());
        }
        if validated
            .insert(file.path, file.content.into_bytes())
            .is_some()
        {
            return Err("evaluation acceptance repeats an expected file".into());
        }
    }
    Ok(validated)
}

fn loop_run_id(key: &str) -> Result<WorkRunId, String> {
    WorkRunId::new(format!("eval-loop-run-{key}")).map_err(|error| error.to_string())
}

fn loop_contract_id(key: &str, suffix: &str) -> Result<WorkContractId, String> {
    WorkContractId::new(format!("eval-loop-contract-{suffix}-{key}"))
        .map_err(|error| error.to_string())
}

fn loop_attempt_id(key: &str, suffix: &str) -> Result<WorkAttemptId, String> {
    WorkAttemptId::new(format!("eval-loop-attempt-{suffix}-{key}"))
        .map_err(|error| error.to_string())
}

fn loop_execution_id(key: &str, suffix: &str) -> Result<WorkExecutionId, String> {
    WorkExecutionId::new(format!("eval-loop-execution-{suffix}-{key}"))
        .map_err(|error| error.to_string())
}

fn loop_command(key: &str, operation: &str) -> Result<CommandId, String> {
    CommandId::new(format!("eval-loop-{operation}-{key}")).map_err(|error| error.to_string())
}

fn short_id(value: &str) -> String {
    ContentDigest::sha256(value.as_bytes())
        .to_string()
        .strip_prefix("sha256:")
        .expect("ContentDigest display uses the sha256 prefix")
        .chars()
        .take(16)
        .collect()
}
