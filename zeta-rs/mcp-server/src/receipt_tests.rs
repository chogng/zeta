use super::*;
use crate::agent::start_fingerprint;
use crate::agent::{AgentOutcomeStatus, StartAgentRequest};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_protocol::TurnId;

static TEMPORARY_ROOT_ID: AtomicU64 = AtomicU64::new(1);

#[test]
fn receipts_replay_results_and_restore_principal_thread_binding_after_reopen() {
    let root = temporary_root();
    let path = root.join("state.sqlite3");
    let request = StartAgentRequest {
        invocation_id: "durable-1".into(),
        prompt: "same".into(),
        timeout: None,
    };
    let fingerprint = start_fingerprint(&request);
    let session_id = SessionId::new("session-durable").unwrap();
    let thread_id = ThreadId::new("thread-durable").unwrap();
    let outcome = AgentOutcome {
        invocation_id: request.invocation_id.clone(),
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        turn_id: TurnId::new("turn-durable").unwrap(),
        status: AgentOutcomeStatus::Completed,
        content: "done".into(),
    };

    {
        let store = ReceiptStore::open(&path).unwrap();
        assert!(matches!(
            store
                .begin("principal-a", &request.invocation_id, fingerprint)
                .unwrap(),
            BeginInvocation::Execute
        ));
        store
            .bind_thread("principal-a", thread_id.clone(), session_id.clone())
            .unwrap();
        store
            .finish(
                "principal-a",
                &request.invocation_id,
                fingerprint,
                Ok(outcome.clone()),
            )
            .unwrap();
    }

    let reopened = ReceiptStore::open(&path).unwrap();
    let replay = reopened
        .begin("principal-a", &request.invocation_id, fingerprint)
        .unwrap();
    assert!(matches!(replay, BeginInvocation::Replay(value) if value == outcome));
    assert_eq!(
        reopened
            .session_for_thread("principal-a", &thread_id)
            .unwrap(),
        Some(session_id)
    );
    assert_eq!(
        reopened
            .session_for_thread("principal-b", &thread_id)
            .unwrap(),
        None
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn running_receipt_is_resumable_after_process_reopen() {
    let root = temporary_root();
    let path = root.join("state.sqlite3");
    let request = StartAgentRequest {
        invocation_id: "running-1".into(),
        prompt: "resume".into(),
        timeout: None,
    };
    let fingerprint = start_fingerprint(&request);
    ReceiptStore::open(&path)
        .unwrap()
        .begin("principal", &request.invocation_id, fingerprint)
        .unwrap();

    let reopened = ReceiptStore::open(&path).unwrap();
    assert!(matches!(
        reopened
            .begin("principal", &request.invocation_id, fingerprint)
            .unwrap(),
        BeginInvocation::Execute
    ));

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn receipt_database_is_private_to_the_host_user() {
    use std::os::unix::fs::PermissionsExt;

    let root = temporary_root();
    let path = root.join("state.sqlite3");
    let store = ReceiptStore::open(&path).unwrap();
    let request = StartAgentRequest {
        invocation_id: "private".into(),
        prompt: "inspect".into(),
        timeout: None,
    };
    assert!(matches!(
        store.begin(
            "principal",
            &request.invocation_id,
            start_fingerprint(&request)
        ),
        Ok(BeginInvocation::Execute)
    ));

    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    std::fs::remove_dir_all(root).unwrap();
}

fn temporary_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-mcp-receipts-{}-{}-{}",
        std::process::id(),
        TEMPORARY_ROOT_ID.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
