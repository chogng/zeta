use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_core::{CreateSessionRequest, CreateSessionThreadRequest, SequenceExpectation};
use zeta_protocol::CommandId;
use zeta_state::StateRuntime;

fn temporary_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-rollout-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn repository_recovers_threads_before_their_session_membership() {
    let root = temporary_root();
    let state = StateRuntime::open(&root).unwrap();
    let repository = LocalStateRepository::open(&state).unwrap();
    let runtime = repository.recover_coordinator().unwrap();
    let session = runtime
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-session").expect("test ID is non-empty"),
            title: "Task".into(),
            model: None,
            workspace: None,
        })
        .unwrap();
    let thread = runtime
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("create-thread").expect("test ID is non-empty"),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(session.sequence),
            title: "Primary branch".into(),
        })
        .unwrap();

    let recovered = LocalStateRepository::open(&state)
        .unwrap()
        .recover_coordinator()
        .unwrap();
    let restored_session = recovered.read_session(&session.session_id).unwrap();
    let restored_thread = recovered.threads().read_thread(&thread.thread_id).unwrap();

    assert_eq!(restored_session.sequence, thread.sequence);
    assert_eq!(restored_session.threads.len(), 1);
    assert_eq!(restored_thread.thread_id, thread.thread_id);
    fs::remove_dir_all(root).unwrap();
}
