use super::CodexAppServerOptions;
use super::CodexAppServerRuntime;
use super::CodexTurnDriver;
use super::CodexTurnExecutionBackend;
use super::CodexTurnExecutionBackendOptions;
use super::CodexTurnWorkspace;
use super::CodexTurnWorkspaceSource;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use zeta_core::CreateThreadRequest;
use zeta_core::InMemoryThreadStore;
use zeta_core::NoThreadUpdates;
use zeta_core::ResolveTurnInteractionRequest;
use zeta_core::SequenceExpectation;
use zeta_core::StartTurnRequest;
use zeta_core::ThreadController;
use zeta_core::TurnExecutionBackend;
use zeta_protocol::ActionApprovalDecision;
use zeta_protocol::ActionApprovalResponse;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentResponse;
use zeta_protocol::ApprovalMode;
use zeta_protocol::CommandId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnExecutionBinding;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
#[cfg(unix)]
fn delegated_turn_streams_and_commits_core_items() {
    let (_root, program) = fake_codex_backend_program(BackendScenario::Complete);
    let (backend, threads, thread_id, turn_id) =
        backend_fixture(program, ApprovalMode::AskPermissions);

    backend.start(&thread_id, &turn_id).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &turn_id) == TurnStatus::Completed);
    wait_until(|| {
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turn_execution_binding
            .is_some()
    });

    let snapshot = threads.read_thread(&thread_id).unwrap();
    assert_eq!(
        snapshot
            .turn_execution_binding
            .as_ref()
            .map(|binding| binding.remote_thread_id.as_str()),
        Some("remote-thread")
    );
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        ThreadItem::Reasoning { turn_id: item_turn_id, text, .. }
            if item_turn_id == &turn_id && text == "Inspecting"
    )));
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        ThreadItem::AgentMessage { turn_id: item_turn_id, text, .. }
            if item_turn_id == &turn_id && text == "Done"
    )));
}

#[test]
#[cfg(unix)]
fn delegated_approval_is_durable_before_upstream_resume() {
    let (_root, program) = fake_codex_backend_program(BackendScenario::Approval);
    let (backend, threads, thread_id, turn_id) =
        backend_fixture(program, ApprovalMode::AskPermissions);

    backend.start(&thread_id, &turn_id).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &turn_id) == TurnStatus::WaitingForApproval);
    let waiting = threads.read_thread(&thread_id).unwrap();
    let interaction = waiting
        .turns
        .iter()
        .find(|turn| turn.turn_id == turn_id)
        .and_then(|turn| turn.pending_interaction.clone())
        .unwrap();
    let AgentRequest::Approval { request } = &interaction.request else {
        panic!("expected durable approval request");
    };
    assert_eq!(
        request.capabilities[0].scope,
        "/tmp/zeta-project: printf approved"
    );

    threads
        .resolve_turn_interaction(
            &thread_id,
            ResolveTurnInteractionRequest {
                command_id: CommandId::new("approve-codex-command").unwrap(),
                expected_sequence: SequenceExpectation::Exact(waiting.sequence),
                turn_id: turn_id.clone(),
                request_id: interaction.request_id,
                response: AgentResponse::Approval {
                    response: ActionApprovalResponse {
                        decision: ActionApprovalDecision::ApproveOnce,
                    },
                },
            },
        )
        .unwrap();
    backend.resume(&thread_id, &turn_id).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &turn_id) == TurnStatus::Completed);

    assert!(
        threads
            .read_thread(&thread_id)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(
                item,
                ThreadItem::AgentMessage { text, .. } if text == "Approved"
            ))
    );
}

#[test]
#[cfg(unix)]
fn reconstructed_backend_resumes_the_persisted_remote_thread() {
    let (root, program) = fake_codex_backend_program(BackendScenario::Complete);
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let thread_id = ThreadId::new("codex-recovered-thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("codex-session").unwrap(),
            thread_id: thread_id.clone(),
            title: "Codex recovery".into(),
        })
        .unwrap();

    let first_turn = start_core_turn(&threads, &thread_id, "start-first-codex-turn");
    let first_backend = build_backend(program.clone(), Arc::clone(&threads));
    first_backend.start(&thread_id, &first_turn).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &first_turn) == TurnStatus::Completed);
    wait_until(|| {
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turn_execution_binding
            .is_some()
    });
    drop(first_backend);

    let second_turn = start_core_turn(&threads, &thread_id, "start-second-codex-turn");
    let recovered_backend = build_backend(program, Arc::clone(&threads));
    recovered_backend.start(&thread_id, &second_turn).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &second_turn) == TurnStatus::Completed);

    let requests = std::fs::read_to_string(root.path().join("requests.log")).unwrap();
    assert_eq!(requests.matches("\"method\":\"thread/start\"").count(), 1);
    assert_eq!(requests.matches("\"method\":\"thread/resume\"").count(), 1);
    assert!(requests.contains("\"threadId\":\"remote-thread\""));
    assert!(requests.contains("\"cwd\":\"/tmp/zeta-project\""));
}

#[test]
#[cfg(unix)]
fn upstream_exit_after_turn_acceptance_fails_without_replay_or_binding() {
    let (root, program) = fake_codex_backend_program(BackendScenario::ExitAfterAcceptance);
    let (backend, threads, thread_id, turn_id) =
        backend_fixture(program, ApprovalMode::AskPermissions);

    backend.start(&thread_id, &turn_id).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &turn_id) == TurnStatus::Failed);
    assert!(
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turn_execution_binding
            .is_none()
    );

    assert!(backend.start(&thread_id, &turn_id).is_err());
    thread::sleep(Duration::from_millis(100));
    let requests = std::fs::read_to_string(root.path().join("requests.log")).unwrap();
    assert_eq!(requests.matches("\"method\":\"turn/start\"").count(), 1);
}

#[test]
fn remote_thread_binding_requires_a_completed_turn_and_is_immutable() {
    let threads = ThreadController::with_store(Arc::new(InMemoryThreadStore::default()));
    let thread_id = ThreadId::new("codex-binding-invariants").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("codex-session").unwrap(),
            thread_id: thread_id.clone(),
            title: "Codex binding".into(),
        })
        .unwrap();
    let turn_id = start_core_turn(&threads, &thread_id, "start-binding-turn");
    let binding = TurnExecutionBinding {
        backend: "codex-app-server".into(),
        remote_thread_id: "remote-thread".into(),
        execution_scope: "test-workspace".into(),
    };

    assert!(
        threads
            .bind_turn_execution(&thread_id, binding.clone())
            .is_err()
    );
    threads
        .complete_turn_without_agent_message(&thread_id, &turn_id)
        .unwrap();
    let first = threads
        .bind_turn_execution(&thread_id, binding.clone())
        .unwrap();
    let replay = threads
        .bind_turn_execution(&thread_id, binding.clone())
        .unwrap();
    assert_eq!(first.sequence, replay.sequence);
    assert!(
        threads
            .bind_turn_execution(
                &thread_id,
                TurnExecutionBinding {
                    backend: "codex-app-server".into(),
                    remote_thread_id: "different-remote-thread".into(),
                    execution_scope: "test-workspace".into(),
                },
            )
            .is_err()
    );
}

#[test]
#[cfg(unix)]
fn in_memory_remote_thread_cannot_cross_workspace_authority() {
    let (root, program) = fake_codex_backend_program(BackendScenario::Complete);
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let thread_id = ThreadId::new("codex-workspace-bound-thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("codex-session").unwrap(),
            thread_id: thread_id.clone(),
            title: "Codex workspace binding".into(),
        })
        .unwrap();
    let workspace = Arc::new(MutableWorkspaceSource::new("scope-a"));
    let backend = build_backend_with_options(
        program,
        Arc::clone(&threads),
        CodexTurnExecutionBackendOptions::from_source(
            workspace.clone(),
            super::CodexThreadAccess::WorkspaceWrite,
        ),
    );

    let first_turn = start_core_turn(&threads, &thread_id, "start-workspace-a-turn");
    backend.start(&thread_id, &first_turn).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &first_turn) == TurnStatus::Completed);
    wait_until(|| {
        threads
            .read_thread(&thread_id)
            .unwrap()
            .turn_execution_binding
            .is_some()
    });

    workspace.set_scope("scope-b");
    let second_turn = start_core_turn(&threads, &thread_id, "start-workspace-b-turn");
    backend.start(&thread_id, &second_turn).unwrap();
    wait_until(|| turn_status(&threads, &thread_id, &second_turn) == TurnStatus::Failed);

    let requests = std::fs::read_to_string(root.path().join("requests.log")).unwrap();
    assert_eq!(requests.matches("\"method\":\"turn/start\"").count(), 1);
}

struct MutableWorkspaceSource {
    scope: Mutex<String>,
}

impl MutableWorkspaceSource {
    fn new(scope: &str) -> Self {
        Self {
            scope: Mutex::new(scope.into()),
        }
    }

    fn set_scope(&self, scope: &str) {
        *self.scope.lock().unwrap() = scope.into();
    }
}

impl CodexTurnWorkspaceSource for MutableWorkspaceSource {
    fn current_workspace(&self) -> Result<CodexTurnWorkspace, zeta_core::CoreError> {
        Ok(CodexTurnWorkspace {
            path: PathBuf::from("/tmp/zeta-project"),
            execution_scope: self.scope.lock().unwrap().clone(),
        })
    }
}

fn backend_fixture(
    program: PathBuf,
    approval_mode: ApprovalMode,
) -> (
    CodexTurnExecutionBackend,
    Arc<ThreadController>,
    ThreadId,
    TurnId,
) {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let thread_id = ThreadId::new("codex-local-thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("codex-session").unwrap(),
            thread_id: thread_id.clone(),
            title: "Codex".into(),
        })
        .unwrap();
    let turn_id =
        start_core_turn_with_approval(&threads, &thread_id, "start-codex-turn", approval_mode);
    let backend = build_backend(program, Arc::clone(&threads));
    (backend, threads, thread_id, turn_id)
}

fn start_core_turn(threads: &ThreadController, thread_id: &ThreadId, command_id: &str) -> TurnId {
    start_core_turn_with_approval(threads, thread_id, command_id, ApprovalMode::AskPermissions)
}

fn start_core_turn_with_approval(
    threads: &ThreadController,
    thread_id: &ThreadId,
    command_id: &str,
    approval_mode: ApprovalMode,
) -> TurnId {
    threads
        .start_turn(
            thread_id,
            StartTurnRequest {
                command_id: CommandId::new(command_id).unwrap(),
                expected_sequence: SequenceExpectation::Any,
                model: None,
                policy_revision: "codex-policy-v1".into(),
                approval_mode,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "inspect the workspace".into(),
                }],
            },
        )
        .unwrap()
        .turn_id
}

fn build_backend(program: PathBuf, threads: Arc<ThreadController>) -> CodexTurnExecutionBackend {
    build_backend_with_options(
        program,
        threads,
        CodexTurnExecutionBackendOptions::workspace_write("/tmp/zeta-project").unwrap(),
    )
}

fn build_backend_with_options(
    program: PathBuf,
    threads: Arc<ThreadController>,
    options: CodexTurnExecutionBackendOptions,
) -> CodexTurnExecutionBackend {
    let runtime = CodexAppServerRuntime::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );
    let (driver, events) = CodexTurnDriver::new(runtime);
    CodexTurnExecutionBackend::new(driver, events, threads, Arc::new(NoThreadUpdates), options)
        .unwrap()
}

fn turn_status(threads: &ThreadController, thread_id: &ThreadId, turn_id: &TurnId) -> TurnStatus {
    threads
        .read_thread(thread_id)
        .unwrap()
        .turns
        .iter()
        .find(|turn| &turn.turn_id == turn_id)
        .unwrap()
        .status
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !predicate() {
        assert!(Instant::now() < deadline, "condition did not become true");
        thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Clone, Copy)]
enum BackendScenario {
    Complete,
    Approval,
    ExitAfterAcceptance,
}

#[cfg(unix)]
fn fake_codex_backend_program(scenario: BackendScenario) -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("codex");
    let request_log = root.path().join("requests.log");
    let turn_events = match scenario {
        BackendScenario::Complete => {
            r#"printf '%s\n' '{"jsonrpc":"2.0","method":"turn/started","params":{"threadId":"remote-thread","turn":{"id":"remote-turn","status":"inProgress","items":[]}}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"item/reasoning/summaryTextDelta","params":{"threadId":"remote-thread","turnId":"remote-turn","itemId":"reasoning-1","delta":"Inspecting","summaryIndex":0}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"item/agentMessage/delta","params":{"threadId":"remote-thread","turnId":"remote-turn","itemId":"message-1","delta":"Done"}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"turn/completed","params":{"threadId":"remote-thread","turn":{"id":"remote-turn","status":"completed","items":[],"error":null}}}'"#
        }
        BackendScenario::Approval => {
            r#"printf '%s\n' '{"jsonrpc":"2.0","method":"turn/started","params":{"threadId":"remote-thread","turn":{"id":"remote-turn","status":"inProgress","items":[]}}}'
      printf '%s\n' '{"jsonrpc":"2.0","id":"approval-backend","method":"item/commandExecution/requestApproval","params":{"threadId":"remote-thread","turnId":"remote-turn","itemId":"command-1","startedAtMs":1700000000000,"reason":"run command","command":"printf approved","cwd":"/tmp/zeta-project"}}'"#
        }
        BackendScenario::ExitAfterAcceptance => {
            r#"printf '%s\n' '{"jsonrpc":"2.0","method":"turn/started","params":{"threadId":"remote-thread","turn":{"id":"remote-turn","status":"inProgress","items":[]}}}'
      exit 0"#
        }
    };
    let approval_response = match scenario {
        BackendScenario::Complete | BackendScenario::ExitAfterAcceptance => "",
        BackendScenario::Approval => {
            r#"    *'"id":"approval-backend"'*)
      case "$line" in
        *'"decision":"accept"'*) ;;
        *) exit 53 ;;
      esac
      printf '%s\n' '{"jsonrpc":"2.0","method":"item/agentMessage/delta","params":{"threadId":"remote-thread","turnId":"remote-turn","itemId":"message-1","delta":"Approved"}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"turn/completed","params":{"threadId":"remote-thread","turn":{"id":"remote-turn","status":"completed","items":[],"error":null}}}'
      ;;"#
        }
    };
    let script = format!(
        r#"#!/bin/sh
while IFS= read -r line; do
  printf '%s\n' "$line" >> '{request_log}'
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"userAgent\":\"codex-test/1.0\",\"codexHome\":\"/tmp/codex-test\",\"platformFamily\":\"unix\",\"platformOs\":\"test\"}}}}"
      ;;
    *'"method":"thread/start"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"thread\":{{\"id\":\"remote-thread\"}}}}}}"
      ;;
    *'"method":"thread/resume"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"thread\":{{\"id\":\"remote-thread\"}}}}}}"
      ;;
    *'"method":"turn/start"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"turn\":{{\"id\":\"remote-turn\",\"status\":\"inProgress\",\"items\":[]}}}}}}"
      {turn_events}
      ;;
{approval_response}
  esac
done
"#,
        request_log = request_log.display(),
    );
    std::fs::write(&program, script).unwrap();
    let mut permissions = std::fs::metadata(&program).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&program, permissions).unwrap();
    (root, program)
}
