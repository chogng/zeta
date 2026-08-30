use crate::CollaborationShape;
use crate::EvalCase;
use crate::EvalFact;
use crate::EvalMode;
use crate::EvalResult;
use crate::case::ExpectedFile;
use crate::development_model::DevelopmentLoopModel;
use crate::development_model::MULTI_SESSION_ALPHA_MARKER;
use crate::development_model::MULTI_SESSION_BETA_MARKER;
use crate::development_model::SINGLE_LOOP_MARKER;
use crate::development_model::TEAM_LOOP_ROOT_MARKER;
use crate::runner::EvalWorkspace;
use crate::runner::elapsed_millis;
use crate::runner::git;
use crate::runner::merge_usage;
use crate::runner::run_key;
use crate::runner::subject;
use crate::runner::tool_call_count;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use zeta_app_server::AppServer;
use zeta_app_server::DevelopmentEvaluationAttempt;
use zeta_app_server::EvaluationExpectedFile;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::MultiSessionEvaluationAgentAttempt;
use zeta_app_server::MultiSessionEvaluationAttemptsRequest;
use zeta_app_server::SessionStateMode;
use zeta_app_server::TeamEvaluationAttemptRequest;
use zeta_app_server::TeamLoopChildRequest;
use zeta_app_server::open_local_app_server;
use zeta_core::ThreadSnapshot;
use zeta_protocol::ContentDigest;
use zeta_protocol::DelegationId;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::PlanStepStatus;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;
use zeta_protocol::WorkAttemptId;
use zeta_work_coordination::IntegrationRootStatus;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkAttemptIntegrationStatus;
use zeta_work_coordination::WorkAttemptVerificationStatus;
use zeta_work_coordination::WorkIntegrationStatus;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRelationStatus;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkVerificationStatus;

const LOOP_TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) fn run_scripted_development_loop(
    case: &EvalCase,
    started: Instant,
) -> Result<EvalResult, String> {
    require_two_file_fixture(case)?;
    match case.collaboration_shape {
        CollaborationShape::SingleAgent => run_single_agent(case, started),
        CollaborationShape::TeamSubagent => run_team(case, started),
        CollaborationShape::MultiSessionAgents => run_multi_session(case, started),
    }
}

fn run_single_agent(case: &EvalCase, started: Instant) -> Result<EvalResult, String> {
    let workspace = EvalWorkspace::new(case)?;
    let baseline_head = git(&workspace.repository, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    let profile = tempfile::tempdir().map_err(|error| error.to_string())?;
    let model = Arc::new(DevelopmentLoopModel::default());
    let server = evaluation_server(profile.path(), &workspace, model)?;
    let host = server.multi_agent_evaluation();
    let key = run_key(case)?;
    let development = host.create_single_agent_development(loop_request(
        case,
        &key,
        format!("[{SINGLE_LOOP_MARKER}] {}", case.task),
        scope(case.expected_files.iter().map(|file| file.path.as_str())),
    ))?;
    host.start_development_agent(&development.agent)?;
    let thread = wait_for_terminal(
        &server,
        &development.agent.thread_id,
        &development.agent.turn_id,
        LOOP_TIMEOUT,
    )?;
    seal_with_retry(
        &host,
        &key,
        &development.work_run_id,
        &development.agent.attempt_id,
    )?;
    let verified = host.verify_development_files(
        &key,
        &development.work_run_id,
        BTreeSet::from([development.agent.attempt_id.clone()]),
        expected_files(case),
    )?;
    let integrated = integrate_if_verified(&host, &key, &development.work_run_id, &verified)?;
    let facts = common_facts(
        case,
        &workspace,
        &baseline_head,
        &integrated,
        &verified.verification_key,
        &[development.agent.clone()],
        &[thread.clone()],
    )?
    .into_iter()
    .chain([(
        "single_root_identity".into(),
        EvalFact::new(
            integrated.participants.len() == 1
                && integrated
                    .participants
                    .get(&development.agent.thread_id)
                    .is_some_and(|participant| {
                        participant.session_id == development.agent.session_id
                            && participant.relation == WorkParticipantRelation::Root
                    })
                && thread.agent_context_seed.is_none(),
            "one root Thread, one root participant, and no delegation seed were checked",
        ),
    )])
    .collect();
    EvalResult::from_facts(
        case,
        subject(
            EvalMode::Scripted,
            development.model,
            "deterministic-single-development-v1",
        ),
        facts,
        thread.usage.clone(),
        tool_call_count(&thread),
        elapsed_millis(started),
    )
}

fn run_team(case: &EvalCase, started: Instant) -> Result<EvalResult, String> {
    let workspace = EvalWorkspace::new(case)?;
    let baseline_head = git(&workspace.repository, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    let profile = tempfile::tempdir().map_err(|error| error.to_string())?;
    let model = Arc::new(DevelopmentLoopModel::default());
    let server = evaluation_server(profile.path(), &workspace, model.clone())?;
    let host = server.multi_agent_evaluation();
    let key = run_key(case)?;
    let coordinator = host.create_team_loop_coordinator(loop_request(
        case,
        &key,
        format!(
            "[{TEAM_LOOP_ROOT_MARKER}] Coordinate this task through two exact child Agents: {}",
            case.task
        ),
        scope(case.expected_files.iter().map(|file| file.path.as_str())),
    ))?;
    host.start_team_loop_coordinator(&coordinator)?;
    if let Err(error) = model.wait_for_team_plan() {
        let _ = model.release_team_plan();
        let _ = model.release_team_children();
        return Err(error);
    }
    let development = match host.bind_team_loop_children(
        &coordinator,
        vec![
            TeamLoopChildRequest {
                delegation_id: DelegationId::new("tool:team-loop-spawn-alpha")
                    .map_err(|error| error.to_string())?,
                title: "alpha-worker".into(),
                task: "[EVAL_TEAM_CHILD_ALPHA] Create alpha.txt with exactly alpha followed by one newline."
                    .into(),
                expected_scope: scope(["alpha.txt"]),
            },
            TeamLoopChildRequest {
                delegation_id: DelegationId::new("tool:team-loop-spawn-beta")
                    .map_err(|error| error.to_string())?,
                title: "beta-worker".into(),
                task: "[EVAL_TEAM_CHILD_BETA] Create beta.txt with exactly beta followed by one newline."
                    .into(),
                expected_scope: scope(["beta.txt"]),
            },
        ],
    ) {
        Ok(development) => development,
        Err(error) => {
            let _ = model.release_team_plan();
            let _ = model.release_team_children();
            return Err(error);
        }
    };
    model.release_team_plan()?;
    if let Err(error) = model.wait_for_team_children(2) {
        let _ = model.release_team_children();
        return Err(error);
    }
    let steered = wait_for_team_messages(&server, &coordinator.root_thread_id, 2, LOOP_TIMEOUT);
    model.release_team_children()?;
    steered?;
    let root_thread = wait_for_terminal(
        &server,
        &coordinator.root_thread_id,
        &coordinator.root_turn_id,
        LOOP_TIMEOUT,
    )?;
    let mut child_threads = Vec::with_capacity(development.children.len());
    for child in &development.children {
        child_threads.push(wait_for_terminal(
            &server,
            &child.thread_id,
            &child.turn_id,
            LOOP_TIMEOUT,
        )?);
        seal_with_retry(&host, &key, &coordinator.work_run_id, &child.attempt_id)?;
    }
    let attempt_ids = development
        .children
        .iter()
        .map(|child| child.attempt_id.clone())
        .collect::<BTreeSet<_>>();
    let verified = host.verify_development_files(
        &key,
        &coordinator.work_run_id,
        attempt_ids,
        expected_files(case),
    )?;
    let integrated = integrate_if_verified(&host, &key, &coordinator.work_run_id, &verified)?;
    let mut all_threads = vec![root_thread.clone()];
    all_threads.extend(child_threads.iter().cloned());
    let expected_delegations = BTreeSet::from([
        DelegationId::new("tool:team-loop-spawn-alpha").map_err(|error| error.to_string())?,
        DelegationId::new("tool:team-loop-spawn-beta").map_err(|error| error.to_string())?,
    ]);
    let root_tool_names = root_thread
        .items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::ToolCall { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let root_plan = root_thread
        .turns
        .iter()
        .find(|turn| turn.turn_id == coordinator.root_turn_id)
        .and_then(|turn| turn.plan.as_ref());
    let exact_plan = root_plan.is_some_and(|plan| {
        plan.steps.len() == 3
            && plan.steps[0].step == "Admit exact alpha.txt and beta.txt child scopes"
            && plan.steps[0].status == PlanStepStatus::InProgress
            && plan.steps[1].status == PlanStepStatus::Pending
            && plan.steps[2].status == PlanStepStatus::Pending
    });
    let team_identity = development.children.iter().all(|child| {
        integrated
            .participants
            .get(&child.thread_id)
            .is_some_and(|participant| {
                participant.session_id == coordinator.root_session_id
                    && matches!(
                        &participant.relation,
                        WorkParticipantRelation::Delegated {
                            parent_thread_id,
                            delegation_id,
                        } if parent_thread_id == &coordinator.root_thread_id
                            && expected_delegations.contains(delegation_id)
                    )
            })
    });
    let facts = common_facts(
        case,
        &workspace,
        &baseline_head,
        &integrated,
        &verified.verification_key,
        &development.children,
        &child_threads,
    )?
    .into_iter()
    .chain([
        (
            "team_root_used_full_coordination_loop".into(),
            EvalFact::new(
                root_tool_names == [
                    "update_plan",
                    "spawn_agent",
                    "spawn_agent",
                    "send_agent_message",
                    "send_agent_message",
                    "wait_agent",
                ] && exact_plan
                    && root_thread.received_delegation_results.len() == 2,
                "the root transcript was checked for plan, admitted spawn, steer, wait/join, and two delivered results",
            ),
        ),
        (
            "same_session_delegated_identity".into(),
            EvalFact::new(
                team_identity
                    && child_threads
                        .iter()
                        .all(|thread| thread.agent_context_seed.is_some()),
                "both child Threads were checked against the same Session and exact delegation identities",
            ),
        ),
        (
            "team_root_completed".into(),
            EvalFact::new(
                turn_status(&root_thread, &coordinator.root_turn_id)
                    == Some(TurnStatus::Completed),
                "the coordinator Turn completed only after its All join was satisfied",
            ),
        ),
    ])
    .collect();
    EvalResult::from_facts(
        case,
        subject(
            EvalMode::Scripted,
            coordinator.model,
            "deterministic-team-development-v1",
        ),
        facts,
        aggregate_usage(&all_threads)?,
        aggregate_tool_calls(&all_threads)?,
        elapsed_millis(started),
    )
}

fn run_multi_session(case: &EvalCase, started: Instant) -> Result<EvalResult, String> {
    let workspace = EvalWorkspace::new(case)?;
    let baseline_head = git(&workspace.repository, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    let profile = tempfile::tempdir().map_err(|error| error.to_string())?;
    let model = Arc::new(DevelopmentLoopModel::default());
    let server = evaluation_server(profile.path(), &workspace, model)?;
    let host = server.multi_agent_evaluation();
    let key = run_key(case)?;
    let attempts = host.create_multi_session_attempts(MultiSessionEvaluationAttemptsRequest {
        run_key: key.clone(),
        objective: case.title.clone(),
        acceptance_conditions: acceptance_conditions(case),
        exclusions: vec!["all paths outside each exact WorkAttempt root".into()],
        first_task: format!("[{MULTI_SESSION_ALPHA_MARKER}] Create only alpha.txt with exact content alpha followed by one newline."),
        second_task: format!("[{MULTI_SESSION_BETA_MARKER}] After the exact wait is satisfied, create only beta.txt with exact content beta followed by one newline."),
        first_expected_scope: scope(["alpha.txt"]),
        second_expected_scope: scope(["beta.txt"]),
    })?;
    let relation_id = host.create_multi_session_wait(
        &key,
        &attempts.work_run_id,
        &attempts.second,
        &attempts.first,
    )?;
    host.start_multi_session_development_agent(&attempts.first)?;
    let first_thread = wait_for_terminal(
        &server,
        &attempts.first.thread_id,
        &attempts.first.turn_id,
        LOOP_TIMEOUT,
    )?;
    let first_sealed = seal_with_retry(
        &host,
        &key,
        &attempts.work_run_id,
        &attempts.first.attempt_id,
    )?;
    let first_result_digest = first_sealed.attempts[&attempts.first.attempt_id]
        .result
        .as_ref()
        .ok_or_else(|| "first independent Session result was not sealed".to_string())?
        .result_digest
        .clone();
    if first_sealed.attempts[&attempts.second.attempt_id].execution_status
        != WorkAttemptExecutionStatus::Writing
    {
        return Err("the exact cross-Session wait did not resume its dependent Attempt".into());
    }
    let second_attempt = host.resume_multi_session_development_agent(
        &key,
        &attempts.second,
        format!("[{MULTI_SESSION_BETA_MARKER}] The exact wait is satisfied. Create only beta.txt with exact content beta followed by one newline."),
    )?;
    let second_thread = wait_for_terminal(
        &server,
        &second_attempt.thread_id,
        &second_attempt.turn_id,
        LOOP_TIMEOUT,
    )?;
    seal_with_retry(
        &host,
        &key,
        &attempts.work_run_id,
        &attempts.second.attempt_id,
    )?;
    let verified = host.verify_development_files(
        &key,
        &attempts.work_run_id,
        BTreeSet::from([
            attempts.first.attempt_id.clone(),
            attempts.second.attempt_id.clone(),
        ]),
        expected_files(case),
    )?;
    let integrated = integrate_if_verified(&host, &key, &attempts.work_run_id, &verified)?;
    let development_attempts = vec![
        development_attempt(&attempts.first),
        development_attempt(&second_attempt),
    ];
    let threads = vec![first_thread.clone(), second_thread.clone()];
    let relation_satisfied = integrated
        .relations
        .get(&relation_id)
        .is_some_and(|relation| {
            relation.source_attempt_id == attempts.second.attempt_id
                && relation.target_attempt_id == attempts.first.attempt_id
                && relation.status
                    == (WorkRelationStatus::Satisfied {
                        evidence_digest: first_result_digest,
                    })
        });
    let independent = attempts.first.session_id != attempts.second.session_id
        && threads
            .iter()
            .all(|thread| thread.agent_context_seed.is_none())
        && [&attempts.first, &attempts.second]
            .into_iter()
            .all(|attempt| {
                integrated
                    .participants
                    .get(&attempt.thread_id)
                    .is_some_and(|participant| {
                        participant.session_id == attempt.session_id
                            && participant.relation == WorkParticipantRelation::Root
                    })
            });
    let facts = common_facts(
        case,
        &workspace,
        &baseline_head,
        &integrated,
        &verified.verification_key,
        &development_attempts,
        &threads,
    )?
    .into_iter()
    .chain([
        (
            "independent_session_identity".into(),
            EvalFact::new(
                independent,
                "two distinct root Sessions and their root participant identities were checked",
            ),
        ),
        (
            "exact_cross_session_wait_satisfied".into(),
            EvalFact::new(
                relation_satisfied,
                "the dependent Session resumed from the exact sealed WorkAttempt and result digest",
            ),
        ),
        (
            "independent_workspaces_isolated".into(),
            EvalFact::new(
                attempts.first.managed_root != attempts.second.managed_root
                    && attempts.first.managed_root != workspace.repository
                    && attempts.second.managed_root != workspace.repository,
                "both Session roots were compared with each other and the source root",
            ),
        ),
    ])
    .collect();
    EvalResult::from_facts(
        case,
        subject(
            EvalMode::Scripted,
            attempts.model,
            "deterministic-multi-session-development-v1",
        ),
        facts,
        aggregate_usage(&threads)?,
        aggregate_tool_calls(&threads)?,
        elapsed_millis(started),
    )
}

fn evaluation_server(
    profile_root: &std::path::Path,
    workspace: &EvalWorkspace,
    model: Arc<DevelopmentLoopModel>,
) -> Result<AppServer, String> {
    open_local_app_server(
        LocalAppServerOptions::new(profile_root)
            .without_built_in_skills()
            .with_session_state_mode(SessionStateMode::Ephemeral)
            .with_dir_root(&workspace.repository)
            .with_agent_model_service(model),
    )
    .map_err(|error| error.to_string())
}

fn loop_request(
    case: &EvalCase,
    run_key: &str,
    task: String,
    expected_scope: WorkScopeClaim,
) -> TeamEvaluationAttemptRequest {
    TeamEvaluationAttemptRequest {
        run_key: run_key.into(),
        objective: case.title.clone(),
        acceptance_conditions: acceptance_conditions(case),
        exclusions: vec!["all paths outside each exact WorkAttempt root".into()],
        task,
        expected_scope,
    }
}

fn acceptance_conditions(case: &EvalCase) -> Vec<String> {
    case.expected_files
        .iter()
        .map(|file| format!("{} has its exact versioned content", file.path))
        .collect()
}

fn expected_files(case: &EvalCase) -> Vec<EvaluationExpectedFile> {
    case.expected_files
        .iter()
        .map(|file| EvaluationExpectedFile {
            path: file.path.clone(),
            content: file.content.clone(),
        })
        .collect()
}

fn scope<'a>(paths: impl IntoIterator<Item = &'a str>) -> WorkScopeClaim {
    WorkScopeClaim {
        components: BTreeSet::from(["multi-agent-development-loop-fixture".into()]),
        paths: paths.into_iter().map(ToOwned::to_owned).collect(),
        contracts: BTreeSet::new(),
        resources: BTreeSet::new(),
    }
}

fn seal_with_retry(
    host: &zeta_app_server::MultiAgentEvaluationHost<'_>,
    run_key: &str,
    work_run_id: &zeta_protocol::WorkRunId,
    attempt_id: &WorkAttemptId,
) -> Result<zeta_work_coordination::WorkRun, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let error = match host.seal_development_attempt(run_key, work_run_id, attempt_id) {
            Ok(run) => return Ok(run),
            Err(error) => error,
        };
        if Instant::now() >= deadline {
            return Err(format!(
                "WorkAttempt {attempt_id} could not be sealed from durable evidence: {error}"
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn integrate_if_verified(
    host: &zeta_app_server::MultiAgentEvaluationHost<'_>,
    run_key: &str,
    work_run_id: &zeta_protocol::WorkRunId,
    verification: &zeta_app_server::EvaluationVerification,
) -> Result<zeta_work_coordination::WorkRun, String> {
    if verification
        .work_run
        .verifications
        .get(&verification.verification_key)
        .is_some_and(|verification| verification.status == WorkVerificationStatus::Verified)
    {
        host.integrate_development_verification(
            run_key,
            work_run_id,
            verification.verification_key.clone(),
        )
    } else {
        Ok(verification.work_run.clone())
    }
}

fn wait_for_terminal(
    server: &AppServer,
    thread_id: &zeta_protocol::ThreadId,
    turn_id: &TurnId,
    timeout: Duration,
) -> Result<ThreadSnapshot, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let thread = server
            .threads()
            .read_thread(thread_id)
            .map_err(|error| error.to_string())?;
        let status = turn_status(&thread, turn_id)
            .ok_or_else(|| format!("evaluation Turn {turn_id} disappeared"))?;
        if matches!(
            status,
            TurnStatus::Completed | TurnStatus::Failed | TurnStatus::Interrupted
        ) {
            return Ok(thread);
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "evaluation Turn {turn_id} did not finish within {} seconds",
                timeout.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_team_messages(
    server: &AppServer,
    root_thread_id: &zeta_protocol::ThreadId,
    expected: usize,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        let thread = server
            .threads()
            .read_thread(root_thread_id)
            .map_err(|error| error.to_string())?;
        if thread.sent_agent_messages.len() >= expected {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Team root delivered only {} of {expected} steering messages",
                thread.sent_agent_messages.len()
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn common_facts(
    case: &EvalCase,
    workspace: &EvalWorkspace,
    baseline_head: &str,
    run: &zeta_work_coordination::WorkRun,
    verification_key: &ContentDigest,
    attempts: &[DevelopmentEvaluationAttempt],
    threads: &[ThreadSnapshot],
) -> Result<BTreeMap<String, EvalFact>, String> {
    let exact = case.expected_files.iter().all(|file| {
        std::fs::read(workspace.repository.join(&file.path))
            .ok()
            .as_deref()
            == Some(file.content.as_bytes())
    });
    let source_clean = git(&workspace.repository, &["status", "--porcelain"])?
        .trim()
        .is_empty();
    let final_head = git(&workspace.repository, &["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    let commit_count = git(
        &workspace.repository,
        &[
            "rev-list",
            "--count",
            &format!("{baseline_head}..{final_head}"),
        ],
    )?
    .trim()
    .parse::<u64>()
    .map_err(|error| error.to_string())?;
    let verification = run.verifications.get(verification_key);
    let verification_exact = verification.is_some_and(|verification| {
        verification.status == WorkVerificationStatus::Verified
            && verification.checks.len() == 4
            && verification.checks.iter().all(|check| {
                check.outcome == zeta_work_coordination::VerificationCheckOutcome::Passed
            })
    });
    let integration = run
        .integrations
        .values()
        .find(|integration| &integration.verification_key == verification_key);
    let integrated = integration.is_some_and(|integration| {
        integration.status == WorkIntegrationStatus::Integrated
            && integration.evidence_digest.is_some()
            && integration
                .roots
                .iter()
                .all(|root| root.status == IntegrationRootStatus::Published)
    });
    let attempts_complete = attempts.iter().all(|expected| {
        run.attempts
            .get(&expected.attempt_id)
            .is_some_and(|attempt| {
                attempt.session_id == expected.session_id
                    && attempt.thread_id == expected.thread_id
                    && attempt.execution_id.as_ref() == Some(&expected.execution_id)
                    && attempt.execution_status == WorkAttemptExecutionStatus::Sealed
                    && attempt.verification_status == WorkAttemptVerificationStatus::Verified
                    && attempt.integration_status == WorkAttemptIntegrationStatus::Integrated
                    && attempt.result.is_some()
            })
    });
    let turns_complete = attempts.iter().all(|attempt| {
        threads
            .iter()
            .find(|thread| thread.thread_id == attempt.thread_id)
            .is_some_and(|thread| {
                turn_status(thread, &attempt.turn_id) == Some(TurnStatus::Completed)
            })
    });
    let roots_isolated = attempts.iter().all(|attempt| {
        attempt.managed_root != workspace.repository
            && attempts
                .iter()
                .filter(|other| other.attempt_id != attempt.attempt_id)
                .all(|other| other.managed_root != attempt.managed_root)
    });
    let final_tree = git(&workspace.repository, &["rev-parse", "HEAD^{tree}"])?
        .trim()
        .to_string();
    Ok(BTreeMap::from([
        (
            "expected_files_integrated_exact".into(),
            EvalFact::new(
                exact,
                "source files were read after integration and compared with the shared case oracle",
            )
            .with_digest(ContentDigest::sha256(final_tree.as_bytes())),
        ),
        (
            "source_git_clean".into(),
            EvalFact::new(
                source_clean,
                "the published source working tree has no staged, unstaged, or untracked residue",
            ),
        ),
        (
            "target_advanced_once".into(),
            EvalFact::new(
                final_head != baseline_head && commit_count == 1,
                format!(
                    "the target advanced by {commit_count} commit after one integration transaction"
                ),
            ),
        ),
        (
            "attempts_completed_full_lifecycle".into(),
            EvalFact::new(
                attempts_complete,
                "every selected WorkAttempt was checked for sealed, verified, and integrated states",
            ),
        ),
        (
            "independent_verification_verified".into(),
            EvalFact::new(
                verification_exact,
                "immutable replay, serializability, external effects, and exact acceptance all passed",
            ),
        ),
        (
            "integration_published".into(),
            EvalFact::new(
                integrated,
                "the exact verified result set and every publication root reached integrated state",
            ),
        ),
        (
            "all_worker_turns_completed".into(),
            EvalFact::new(
                turns_complete,
                "every selected worker Turn reached the completed terminal state",
            ),
        ),
        (
            "work_attempt_roots_isolated".into(),
            EvalFact::new(
                roots_isolated,
                "each writer used a distinct managed root outside the source checkout",
            ),
        ),
    ]))
}

fn development_attempt(
    attempt: &MultiSessionEvaluationAgentAttempt,
) -> DevelopmentEvaluationAttempt {
    DevelopmentEvaluationAttempt {
        session_id: attempt.session_id.clone(),
        thread_id: attempt.thread_id.clone(),
        turn_id: attempt.turn_id.clone(),
        attempt_id: attempt.attempt_id.clone(),
        execution_id: attempt.execution_id.clone(),
        managed_root: attempt.managed_root.clone(),
    }
}

fn turn_status(thread: &ThreadSnapshot, turn_id: &TurnId) -> Option<TurnStatus> {
    thread
        .turns
        .iter()
        .find(|turn| &turn.turn_id == turn_id)
        .map(|turn| turn.status)
}

fn aggregate_usage(threads: &[ThreadSnapshot]) -> Result<ModelUsageSummary, String> {
    let mut usage = ModelUsageSummary::default();
    for thread in threads {
        usage = merge_usage(&usage, &thread.usage)?;
    }
    Ok(usage)
}

fn aggregate_tool_calls(threads: &[ThreadSnapshot]) -> Result<u64, String> {
    threads.iter().try_fold(0_u64, |total, thread| {
        total
            .checked_add(tool_call_count(thread))
            .ok_or_else(|| "development evaluation Tool Call count overflowed".into())
    })
}

fn expected_file<'a>(case: &'a EvalCase, path: &str) -> Result<&'a ExpectedFile, String> {
    case.expected_files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| format!("development loop case {} omitted {path}", case.id))
}

fn require_two_file_fixture(case: &EvalCase) -> Result<(), String> {
    let _ = expected_file(case, "alpha.txt")?;
    let _ = expected_file(case, "beta.txt")?;
    Ok(())
}
