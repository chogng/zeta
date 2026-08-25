use std::sync::mpsc;
use std::time::Duration;

use super::EndpointPaths;

#[test]
fn profile_operation_lock_serializes_mutating_lifecycle_commands() {
    let profile = tempfile::tempdir().unwrap();
    let endpoint = EndpointPaths::prepare(profile.path()).unwrap();
    let first = endpoint.acquire_operation_lock().unwrap();
    let contender = endpoint.clone();
    let (acquired, receive) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let second = contender.acquire_operation_lock().unwrap();
        acquired.send(()).unwrap();
        drop(second);
    });

    assert!(receive.recv_timeout(Duration::from_millis(100)).is_err());
    drop(first);
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.join().unwrap();
}

#[test]
fn abandoned_operation_lock_is_recovered_after_its_heartbeat_expires() {
    let profile = tempfile::tempdir().unwrap();
    let endpoint = EndpointPaths::prepare(profile.path()).unwrap();
    std::fs::create_dir(&endpoint.operation_lock).unwrap();
    std::fs::write(endpoint.operation_lock.join("heartbeat"), "abandoned").unwrap();
    std::thread::sleep(Duration::from_millis(300));

    let recovered = endpoint.acquire_operation_lock().unwrap();

    drop(recovered);
    assert!(!endpoint.operation_lock.exists());
}
