use super::CodexAppServerOptions;
use super::CodexAppServerRuntime;
use super::CodexApprovalDecision;
use super::CodexTurnDriver;
use super::CodexTurnErrorKind;
use super::CodexTurnEvent;
use super::CodexTurnStatus;
use super::CodexUserInputAnswers;
use super::StartCodexThread;
use super::StartCodexTurn;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
#[cfg(unix)]
fn read_only_turn_streams_typed_events_and_completes() {
    let (_root, program) = fake_codex_turn_program();
    let runtime = CodexAppServerRuntime::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );
    let (driver, events) = CodexTurnDriver::new(runtime);
    let thread = driver
        .start_thread(
            &StartCodexThread::read_only(std::path::Path::new("/tmp/zeta-project"))
                .unwrap()
                .with_model("gpt-test")
                .unwrap(),
        )
        .unwrap();
    assert_eq!(thread.as_str(), "thread-1");
    let turn = driver
        .start_turn(&StartCodexTurn::text(thread.clone(), "inspect the repo").unwrap())
        .unwrap();
    assert_eq!(turn.as_str(), "turn-1");

    assert_eq!(
        events.recv_timeout(Duration::from_secs(10)).unwrap(),
        CodexTurnEvent::Started {
            thread_id: thread.clone(),
            turn_id: turn.clone(),
        }
    );
    assert_eq!(
        events.recv_timeout(Duration::from_secs(10)).unwrap(),
        CodexTurnEvent::ReasoningSummaryDelta {
            thread_id: thread.clone(),
            turn_id: turn.clone(),
            item_id: "reasoning-1".into(),
            delta: "Inspecting".into(),
        }
    );
    assert_eq!(
        events.recv_timeout(Duration::from_secs(10)).unwrap(),
        CodexTurnEvent::AgentMessageDelta {
            thread_id: thread.clone(),
            turn_id: turn.clone(),
            item_id: "message-1".into(),
            delta: "Done".into(),
        }
    );
    assert_eq!(
        events.recv_timeout(Duration::from_secs(10)).unwrap(),
        CodexTurnEvent::Completed {
            thread_id: thread.clone(),
            turn_id: turn.clone(),
            status: CodexTurnStatus::Completed,
        }
    );
    driver.interrupt(&thread, &turn).unwrap();
}

#[test]
fn read_only_thread_requires_an_absolute_utf8_working_directory() {
    assert!(StartCodexThread::read_only(std::path::Path::new("relative")).is_err());
}

#[test]
#[cfg(unix)]
fn workspace_write_turn_resolves_approval_and_user_input_once() {
    let (_root, program) = fake_codex_interaction_program();
    let runtime = CodexAppServerRuntime::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );
    let (driver, events) = CodexTurnDriver::new(runtime);
    let thread = driver
        .start_thread(
            &StartCodexThread::workspace_write(std::path::Path::new("/tmp/zeta-project")).unwrap(),
        )
        .unwrap();
    let turn = driver
        .start_turn(&StartCodexTurn::text(thread.clone(), "update the file").unwrap())
        .unwrap();

    assert!(matches!(
        events.recv_timeout(Duration::from_secs(10)).unwrap(),
        CodexTurnEvent::Started { .. }
    ));
    let CodexTurnEvent::CommandApprovalRequested(approval) =
        events.recv_timeout(Duration::from_secs(10)).unwrap()
    else {
        panic!("expected command approval request");
    };
    assert_eq!(approval.thread_id, thread);
    assert_eq!(approval.turn_id, turn);
    assert_eq!(approval.item_id, "command-1");
    assert_eq!(approval.command, "printf approved");
    assert_eq!(approval.cwd.as_deref(), Some("/tmp/zeta-project"));
    assert_eq!(
        approval.available_decisions,
        vec![
            CodexApprovalDecision::Accept,
            CodexApprovalDecision::AcceptForSession,
            CodexApprovalDecision::Decline,
        ]
    );
    driver
        .resolve_approval(
            &approval.request_id,
            CodexApprovalDecision::AcceptForSession,
        )
        .unwrap();
    assert_eq!(
        driver
            .resolve_approval(&approval.request_id, CodexApprovalDecision::Decline)
            .unwrap_err()
            .kind(),
        CodexTurnErrorKind::Conflict
    );

    let CodexTurnEvent::UserInputRequested(input) =
        events.recv_timeout(Duration::from_secs(10)).unwrap()
    else {
        panic!("expected user-input request");
    };
    assert!(input.is_blocking);
    assert_eq!(input.questions.len(), 1);
    assert_eq!(input.questions[0].id, "scope");
    assert_eq!(input.questions[0].options[0].label, "Current file");
    let answers = CodexUserInputAnswers::new()
        .answer("scope", vec!["Current file".into()])
        .unwrap();
    driver
        .submit_user_input(&input.request_id, &answers)
        .unwrap();
    assert_eq!(
        events.recv_timeout(Duration::from_secs(10)).unwrap(),
        CodexTurnEvent::Completed {
            thread_id: thread,
            turn_id: turn,
            status: CodexTurnStatus::Completed,
        }
    );
}

#[cfg(unix)]
fn fake_codex_turn_program() -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("codex");
    std::fs::write(
        &program,
        r#"#!/bin/sh
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"userAgent\":\"codex-test/1.0\",\"codexHome\":\"/tmp/codex-test\",\"platformFamily\":\"unix\",\"platformOs\":\"test\"}}"
      ;;
    *'"method":"thread/start"'*)
      case "$line" in
        *'"approvalPolicy":"never"'*) ;;
        *) exit 31 ;;
      esac
      case "$line" in
        *'"sandbox":"read-only"'*) ;;
        *) exit 31 ;;
      esac
      case "$line" in
        *'"cwd":"/tmp/zeta-project"'*) ;;
        *) exit 31 ;;
      esac
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"thread\":{\"id\":\"thread-1\"}}}"
      ;;
    *'"method":"turn/start"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"turn\":{\"id\":\"turn-1\",\"status\":\"inProgress\",\"items\":[]}}}"
      printf '%s\n' '{"jsonrpc":"2.0","method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","status":"inProgress","items":[]}}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"item/reasoning/summaryTextDelta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"reasoning-1","delta":"Inspecting","summaryIndex":0}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"message-1","delta":"Done"}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","status":"completed","items":[],"error":null}}}'
      ;;
    *'"method":"turn/interrupt"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}"
      ;;
  esac
done
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&program).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&program, permissions).unwrap();
    (root, program)
}

#[cfg(unix)]
fn fake_codex_interaction_program() -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("codex");
    std::fs::write(
        &program,
        r#"#!/bin/sh
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"userAgent\":\"codex-test/1.0\",\"codexHome\":\"/tmp/codex-test\",\"platformFamily\":\"unix\",\"platformOs\":\"test\"}}"
      ;;
    *'"method":"thread/start"'*)
      case "$line" in
        *'"approvalPolicy":"on-request"'*) ;;
        *) exit 41 ;;
      esac
      case "$line" in
        *'"sandbox":"workspace-write"'*) ;;
        *) exit 41 ;;
      esac
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"thread\":{\"id\":\"thread-write\"}}}"
      ;;
    *'"method":"turn/start"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"turn\":{\"id\":\"turn-write\",\"status\":\"inProgress\",\"items\":[]}}}"
      printf '%s\n' '{"jsonrpc":"2.0","method":"turn/started","params":{"threadId":"thread-write","turn":{"id":"turn-write","status":"inProgress","items":[]}}}'
      printf '%s\n' '{"jsonrpc":"2.0","id":"approval-1","method":"item/commandExecution/requestApproval","params":{"threadId":"thread-write","turnId":"turn-write","itemId":"command-1","startedAtMs":1700000000000,"reason":"run requested command","command":"printf approved","cwd":"/tmp/zeta-project","availableDecisions":["accept","acceptForSession","decline"]}}'
      ;;
    *'"id":"approval-1"'*)
      case "$line" in
        *'"decision":"acceptForSession"'*) ;;
        *) exit 42 ;;
      esac
      printf '%s\n' '{"jsonrpc":"2.0","id":700,"method":"item/tool/requestUserInput","params":{"threadId":"thread-write","turnId":"turn-write","itemId":"input-1","questions":[{"id":"scope","header":"Scope","question":"Which scope?","isOther":false,"isSecret":false,"options":[{"label":"Current file","description":"Only update the current file"}]}],"isBlocking":true}}'
      ;;
    *'"id":700'*)
      case "$line" in
        *'"scope":{"answers":["Current file"]}'*) ;;
        *) exit 43 ;;
      esac
      printf '%s\n' '{"jsonrpc":"2.0","method":"turn/completed","params":{"threadId":"thread-write","turn":{"id":"turn-write","status":"completed","items":[],"error":null}}}'
      ;;
  esac
done
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&program).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&program, permissions).unwrap();
    (root, program)
}
