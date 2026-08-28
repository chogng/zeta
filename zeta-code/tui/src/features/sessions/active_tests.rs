use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::InProcessClientOptions;
use zeta_app_server_client::InProcessTransport;
use zeta_app_server_client::start_in_process_client;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_client::ClientError;
use zeta_client::ClientRequest;
use zeta_client::ClientResponse;
use zeta_client::OperationClient;

use super::ActiveConversation;
use super::workspace_reconnect;
use crate::TuiRecoveryState;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkspaceBinding;
use zeta_protocol::WorkspaceTrustId;

#[test]
fn foreign_workspace_session_requests_host_reconnect() {
    let current = bound_session("current", "/workspaces/current", '1');
    let target = bound_session("target", "/workspaces/target", '2');
    let thread_id = ThreadId::new("target-thread").unwrap();

    let reconnect = workspace_reconnect(&current, &target, &thread_id)
        .unwrap()
        .unwrap();

    assert_eq!(
        reconnect.workspace_root(),
        std::path::Path::new("/workspaces/target")
    );
    assert_eq!(reconnect.recovery().session_id(), &target.session_id);
    assert_eq!(reconnect.recovery().thread_id(), &thread_id);
    assert!(
        workspace_reconnect(&target, &target, &thread_id)
            .unwrap()
            .is_none()
    );
}

fn bound_session(id: &str, root: &str, digest: char) -> Session {
    Session {
        session_id: SessionId::new(id).unwrap(),
        title: id.into(),
        status: SessionStatus::Active,
        model: None,
        workspace: Some(WorkspaceBinding {
            authority_id: format!("sha256:{}", digest.to_string().repeat(64))
                .parse::<WorkspaceTrustId>()
                .unwrap(),
            root: root.into(),
        }),
        next_approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
        sequence: 1,
        threads: Vec::new(),
    }
}

#[test]
fn recovery_reopens_the_exact_durable_conversation() {
    let (mut client, state_root) = client();
    let conversation = ActiveConversation::start(&mut client, "recover exact".into()).unwrap();
    let state = TuiRecoveryState::new(
        conversation.session_id().clone(),
        conversation.thread_id().clone(),
    );

    let recovered = ActiveConversation::recover(&mut client, state).unwrap();

    assert_eq!(recovered.session_id(), conversation.session_id());
    assert_eq!(recovered.thread_id(), conversation.thread_id());
    assert_eq!(recovered.thread_sequence(), conversation.thread_sequence());
    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn recovery_uses_the_latest_active_thread_when_the_preferred_thread_is_missing() {
    let (mut client, state_root) = client();
    let mut conversation =
        ActiveConversation::start(&mut client, "recover fallback".into()).unwrap();
    let session_id = conversation.session_id().clone();
    conversation
        .fork_active_thread(&mut client, "surviving thread")
        .unwrap();
    let active_thread_id = conversation.thread_id().clone();

    let recovered = ActiveConversation::recover(
        &mut client,
        TuiRecoveryState::new(session_id, ThreadId::new("missing-thread").unwrap()),
    )
    .unwrap();

    assert_eq!(recovered.thread_id(), &active_thread_id);
    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

fn client() -> (AppServerClient<InProcessTransport>, PathBuf) {
    static NEXT_STATE_ROOT: AtomicU64 = AtomicU64::new(1);
    let state_root = std::env::temp_dir().join(format!(
        "zeta-tui-recovery-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_STATE_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    let client = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "zeta-tui-recovery-test".into(),
                version: "1".into(),
            },
        )
        .with_model_operation_client(Arc::new(OfflineOperationClient)),
    )
    .unwrap();
    (client, state_root)
}

struct OfflineOperationClient;

impl OperationClient for OfflineOperationClient {
    fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
        Err(ClientError::Transport(
            "model transport is disabled in TUI recovery tests".into(),
        ))
    }
}
