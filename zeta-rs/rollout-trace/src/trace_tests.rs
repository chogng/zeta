use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_core::CreateThreadRequest;
use zeta_protocol::{SessionId, ThreadId};
use zeta_rollout::LocalStateRepository;
use zeta_state::StateRuntime;

fn temporary_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-rollout-trace-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn trace_groups_thread_streams_by_session_id() {
    let root = temporary_root();
    let state = StateRuntime::open(&root).unwrap();
    let repository = LocalStateRepository::open(&state).unwrap();
    let session_id = SessionId::new("session_1").expect("test ID is non-empty");
    let thread_id = ThreadId::new("thread_1").expect("test ID is non-empty");
    let thread_store = repository.thread_store();
    repository
        .recover_threads()
        .unwrap()
        .create_thread(CreateThreadRequest {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            title: "Primary branch".into(),
        })
        .unwrap();

    let trace = capture_session_trace(thread_store.as_ref(), &session_id).unwrap();

    assert_eq!(trace.format_version, ROLLOUT_TRACE_FORMAT_VERSION);
    assert_eq!(trace.threads[0].events[0].sequence, 1);
    assert_eq!(trace.threads[0].thread_id, thread_id);
    fs::remove_dir_all(root).unwrap();
}
