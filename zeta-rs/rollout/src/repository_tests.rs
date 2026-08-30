use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_core::CreateThreadRequest;
use zeta_protocol::{SessionId, ThreadId};
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
fn repository_keeps_idle_history_lazy_and_loads_it_on_access() {
    let root = temporary_root();
    let state = StateRuntime::open(&root).unwrap();
    let repository = LocalStateRepository::open(&state).unwrap();
    let threads = repository.recover_threads().unwrap();
    let session_id = SessionId::new("session-1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread-1").expect("test ID is non-empty");
    let created = threads
        .create_thread(CreateThreadRequest {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            title: "Primary branch".into(),
        })
        .unwrap();

    let recovered = LocalStateRepository::open(&state)
        .unwrap()
        .recover_threads()
        .unwrap();
    assert!(recovered.list_loaded_threads().unwrap().is_empty());
    assert_eq!(recovered.list_thread_catalog().unwrap().len(), 1);

    let restored = recovered.read_thread(&thread_id).unwrap();

    assert_eq!(restored.session_id, session_id);
    assert_eq!(restored.thread_id, thread_id);
    assert_eq!(restored.title, "Primary branch");
    assert_eq!(restored.sequence, created.sequence);
    assert_eq!(recovered.list_loaded_threads().unwrap().len(), 1);
    fs::remove_dir_all(root).unwrap();
}
