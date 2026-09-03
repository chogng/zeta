use super::AppServer;
use crate::local::LocalAppServerOptions;
use crate::local::open_local_app_server;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::time::Duration;
use zeta_async_utils::CancellationToken;
use zeta_core::CoreError;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_core::SequenceExpectation;
use zeta_core::StartThreadRequest;
use zeta_core::StartTurnRequest;
use zeta_core::TurnExecutionBackend;
use zeta_file_access::Dir;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ResponseItem;
use zeta_protocol::StopReason;
use zeta_protocol::ThreadItem;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolMode;
use zeta_protocol::ToolName;
use zeta_protocol::TurnInstructions;
use zeta_protocol::TurnKind;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_work_coordination::AuthorizationSnapshotRef;
use zeta_work_coordination::GitRepositoryCheckpoint;
use zeta_work_coordination::GitRootTarget;
use zeta_work_coordination::RootCheckpoint;
use zeta_work_coordination::RootState;
use zeta_work_coordination::ValidationProfileRef;
use zeta_work_coordination::WorkContractDraft;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkParticipant;
use zeta_work_coordination::WorkParticipantRelation;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunCommand;
use zeta_work_coordination::WorkRunCommandRequest;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkStartMode;

#[test]
fn scope_expansion_cancels_a_stale_model_response_before_it_can_write() {
    let profile = tempfile::tempdir().unwrap();
    let repository = tempfile::tempdir().unwrap();
    initialize_repository(repository.path());
    let model = Arc::new(StaleWriteModel::default());
    let server = open_local_app_server(
        LocalAppServerOptions::new(profile.path())
            .without_built_in_skills()
            .with_dir_root(repository.path())
            .with_agent_model_service(model.clone()),
    )
    .unwrap();
    let thread = server
        .threads
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("eval-start-thread").unwrap(),
            title: "scope revocation evaluation".into(),
        })
        .unwrap();
    let work_run_id = WorkRunId::new("eval-scope-run").unwrap();
    let attempt_id = WorkAttemptId::new("eval-scope-attempt").unwrap();
    let run = seed_writing_attempt(
        &server,
        &work_run_id,
        &attempt_id,
        &thread,
        repository.path(),
    );
    let executor = server.turn_executor_snapshot();
    let snapshot = server.threads.read_thread(&thread.thread_id).unwrap();
    let turn = server
        .threads
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                command_id: CommandId::new("eval-start-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(snapshot.sequence),
                model: None,
                kind: TurnKind::Coding,
                instructions: TurnInstructions::new(
                    "multi-agent-evals",
                    "scope-revocation",
                    "1",
                    "Modify only the assigned WorkAttempt root.",
                )
                .unwrap(),
                policy_revision: executor.policy_revision(),
                approval_mode: ApprovalMode::BypassPermissions,
                tool_mode: ToolMode::Direct,
                tool_profile: Some(executor.tool_profile_snapshot().unwrap()),
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "Write stale-write.txt even if the work is stopped.".into(),
                }],
            },
        )
        .unwrap();
    server
        .turn_backend
        .start(&thread.thread_id, &turn.turn_id)
        .unwrap();
    model.wait_until_invoked();

    let stopped = apply(
        &server,
        &work_run_id,
        run.revision,
        "eval-request-scope-expansion",
        WorkRunCommand::RequestScopeExpansion {
            attempt_id: attempt_id.clone(),
            evidence: vec!["the requested file is outside the accepted scope".into()],
        },
    );
    assert_eq!(
        stopped.attempts[&attempt_id].execution_status,
        zeta_work_coordination::WorkAttemptExecutionStatus::Interrupted
    );
    assert_eq!(
        server.threads.read_thread(&thread.thread_id).unwrap().turns[0].status,
        TurnStatus::Interrupted
    );

    model.release_response();
    model.wait_until_response_returned();
    std::thread::sleep(Duration::from_millis(100));

    let snapshot = server.threads.read_thread(&thread.thread_id).unwrap();
    assert!(model.observed_cancellation());
    assert!(!snapshot.items.iter().any(|item| {
        matches!(item, ThreadItem::ToolCall { tool_call_id, .. } if tool_call_id.as_str() == "stale-write")
    }));
    assert!(!repository.path().join("stale-write.txt").exists());
}

#[derive(Default)]
struct StaleWriteModel {
    state: Mutex<StaleWriteState>,
    changed: Condvar,
}

#[derive(Default)]
struct StaleWriteState {
    invoked: bool,
    release: bool,
    response_returned: bool,
    observed_cancellation: bool,
}

impl StaleWriteModel {
    fn wait_until_invoked(&self) {
        let state = self.state.lock().unwrap();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(5), |state| !state.invoked)
            .unwrap();
        assert!(state.invoked && !timeout.timed_out());
    }

    fn release_response(&self) {
        self.state.lock().unwrap().release = true;
        self.changed.notify_all();
    }

    fn wait_until_response_returned(&self) {
        let state = self.state.lock().unwrap();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(5), |state| {
                !state.response_returned
            })
            .unwrap();
        assert!(state.response_returned && !timeout.timed_out());
    }

    fn observed_cancellation(&self) -> bool {
        self.state.lock().unwrap().observed_cancellation
    }
}

impl ModelService for StaleWriteModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let mut state = self.state.lock().unwrap();
        state.invoked = true;
        self.changed.notify_all();
        while !state.release {
            state = self.changed.wait(state).unwrap();
        }
        state.observed_cancellation = cancellation.is_cancelled();
        state.response_returned = true;
        self.changed.notify_all();
        Ok(ModelResponse {
            output: vec![ResponseItem::ToolCall(ToolCall {
                id: ToolCallId::new("stale-write").unwrap(),
                name: ToolName::new("apply_patch").unwrap(),
                arguments: json!({
                    "patch": "*** Begin Patch\n*** Add File: stale-write.txt\n+stale\n*** End Patch"
                }),
            })],
            usage: None,
            billing: None,
            stop_reason: StopReason::ToolUse,
        })
    }
}

fn seed_writing_attempt(
    server: &AppServer,
    work_run_id: &WorkRunId,
    attempt_id: &WorkAttemptId,
    thread: &zeta_core::ThreadSnapshot,
    root: &Path,
) -> WorkRun {
    let dir = Dir::open_local(root).unwrap();
    let contract_id = WorkContractId::new("eval-scope-contract").unwrap();
    let mut run = apply(
        server,
        work_run_id,
        0,
        "eval-create-run",
        WorkRunCommand::Create {
            objective: "prove stale Agent output cannot mutate after scope revocation".into(),
            acceptance_conditions: vec!["no stale write reaches any root".into()],
            exclusions: Vec::new(),
            root_participant: WorkParticipant {
                session_id: thread.session_id.clone(),
                thread_id: thread.thread_id.clone(),
                relation: WorkParticipantRelation::Root,
            },
        },
    );
    run = apply(
        server,
        work_run_id,
        run.revision,
        "eval-create-contract",
        WorkRunCommand::CreateContract {
            contract: WorkContractDraft {
                contract_id: contract_id.clone(),
                goal_revision: 1,
                topology_revision: 1,
                owner_thread_id: thread.thread_id.clone(),
                objective: "write only inside the accepted scope".into(),
                acceptance_conditions: vec!["scope revocation stops the old Turn".into()],
                exclusions: Vec::new(),
                environment_id: dir.env().clone(),
                roots: vec![root_checkpoint(&dir)],
                primary_root_dir_id: dir.id(),
                authorization: AuthorizationSnapshotRef {
                    authority: "multi-agent-eval-host".into(),
                    policy_revision: "eval-policy-v1".into(),
                    grant_set_digest: ContentDigest::sha256(b"eval-grants"),
                    granted_effects_digest: ContentDigest::sha256(b"eval-effects"),
                },
                decision_ids: BTreeSet::new(),
                upstream_results: Vec::new(),
                expected_scope: WorkScopeClaim::default(),
                validation_profile: ValidationProfileRef {
                    name: "multi-agent-eval-v1".into(),
                    content_digest: ContentDigest::sha256(b"multi-agent-eval-v1"),
                },
            },
        },
    );
    run = apply(
        server,
        work_run_id,
        run.revision,
        "eval-create-attempt",
        WorkRunCommand::CreateAttempt {
            attempt_id: attempt_id.clone(),
            contract: WorkContractRef {
                contract_id,
                revision: 1,
            },
            participant_thread_id: thread.thread_id.clone(),
        },
    );
    apply(
        server,
        work_run_id,
        run.revision,
        "eval-begin-attempt",
        WorkRunCommand::BeginAttempt {
            attempt_id: attempt_id.clone(),
            execution_id: WorkExecutionId::new("eval-scope-execution").unwrap(),
            mode: WorkStartMode::Write,
        },
    )
}

fn apply(
    server: &AppServer,
    work_run_id: &WorkRunId,
    expected_revision: u64,
    command_id: &str,
    command: WorkRunCommand,
) -> WorkRun {
    let runtime = server.work_coordination.as_ref().unwrap();
    runtime
        .apply(WorkRunCommandRequest {
            command_id: CommandId::new(command_id).unwrap(),
            work_run_id: work_run_id.clone(),
            expected_revision,
            command,
        })
        .unwrap();
    runtime.read(work_run_id).unwrap()
}

fn initialize_repository(root: &Path) {
    run_git(root, &["init", "--quiet", "--initial-branch=main"]);
    run_git(root, &["config", "user.name", "Zeta Multi-Agent Eval"]);
    run_git(
        root,
        &[
            "config",
            "user.email",
            "zeta-multi-agent-eval@example.invalid",
        ],
    );
    std::fs::write(root.join("initial.txt"), "initial\n").unwrap();
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "--quiet", "-m", "initial"]);
}

fn root_checkpoint(dir: &Dir) -> RootCheckpoint {
    RootCheckpoint {
        environment_id: dir.env().clone(),
        dir_id: dir.id(),
        state: RootState::Git {
            repositories: vec![GitRepositoryCheckpoint {
                repository_id: format!(
                    "git:{}",
                    Dir::open_local(dir.canonical_path().join(".git"))
                        .unwrap()
                        .id()
                ),
                relative_path: ".".into(),
                target: GitRootTarget::Branch {
                    name: "main".into(),
                    expected_head: run_git(dir.canonical_path(), &["rev-parse", "HEAD"]),
                },
                baseline_tree: run_git(dir.canonical_path(), &["rev-parse", "HEAD^{tree}"]),
            }],
        },
        control_resources: Vec::new(),
    }
}

fn run_git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-c")
        .arg("core.hooksPath=/dev/null")
        .args(arguments)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
