use std::fs;
use std::fs::OpenOptions;

use serde_json::json;
use tempfile::TempDir;
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;

use crate::RemoteConnectionProfileStore;
use crate::RemoteConnectionProfileStoreFailureKind;

#[test]
fn activation_is_atomic_idempotent_and_retains_one_previous_runtime() {
    let directory = TempDir::new().unwrap();
    let store = store(&directory);
    let target = target();
    let system = profile(target.clone(), "zeta");
    let first = profile(target.clone(), "/runtime/one/bin/zeta");
    let second = profile(target.clone(), "/runtime/two/bin/zeta");

    store.activate(&system).unwrap();
    let activated = store.activate(&first).unwrap();
    assert_eq!(
        activated.active_runtime().executable(),
        "/runtime/one/bin/zeta"
    );
    assert_eq!(activated.previous_runtime().unwrap().executable(), "zeta");
    store.activate(&first).unwrap();
    let activated = store.activate(&second).unwrap();
    assert_eq!(
        activated.active_runtime().executable(),
        "/runtime/two/bin/zeta"
    );
    assert_eq!(
        activated.previous_runtime().unwrap().executable(),
        "/runtime/one/bin/zeta"
    );

    let reopened = RemoteConnectionProfileStore::new(store.path());
    let loaded = reopened.connection(&target).unwrap().unwrap();
    assert_eq!(loaded, activated);
    assert_eq!(reopened.connections().unwrap(), vec![activated]);
    let encoded = fs::read_to_string(store.path()).unwrap();
    assert!(!encoded.contains("password"));
    assert!(!encoded.contains("privateKey"));
    assert_eq!(
        fs::read_dir(directory.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
            .count(),
        0
    );
}

#[test]
fn profile_root_constructor_uses_the_shared_remote_resource() {
    let directory = TempDir::new().unwrap();
    let store = RemoteConnectionProfileStore::from_profile_root(directory.path());

    assert_eq!(
        store.path(),
        directory.path().join("remote/connections.json")
    );
}

#[test]
fn rollback_swaps_generations_and_persists_the_result() {
    let directory = TempDir::new().unwrap();
    let store = store(&directory);
    let target = target();
    store
        .activate(&profile(target.clone(), "/runtime/one/bin/zeta"))
        .unwrap();
    store
        .activate(&profile(target.clone(), "/runtime/two/bin/zeta"))
        .unwrap();

    let previous = profile(target.clone(), "/runtime/one/bin/zeta");
    let rolled_back = store
        .rollback_to_verified(&previous, &previous)
        .unwrap()
        .unwrap();
    assert_eq!(
        rolled_back.active_runtime().executable(),
        "/runtime/one/bin/zeta"
    );
    assert_eq!(
        rolled_back.previous_runtime().unwrap().executable(),
        "/runtime/two/bin/zeta"
    );
    assert_eq!(
        store
            .connection(&target)
            .unwrap()
            .unwrap()
            .active_runtime()
            .executable(),
        "/runtime/one/bin/zeta"
    );
}

#[test]
fn rollback_does_not_swap_a_generation_that_changed_after_validation() {
    let directory = TempDir::new().unwrap();
    let store = store(&directory);
    let target = target();
    let first = profile(target.clone(), "/runtime/one/bin/zeta");
    let second = profile(target.clone(), "/runtime/two/bin/zeta");
    let third = profile(target.clone(), "/runtime/three/bin/zeta");
    store.activate(&first).unwrap();
    store.activate(&second).unwrap();

    store.activate(&third).unwrap();

    assert_eq!(store.rollback_to_verified(&first, &first).unwrap(), None);
    let current = store.connection(&target).unwrap().unwrap();
    assert_eq!(current.active_runtime(), third.runtime());
    assert_eq!(current.previous_runtime(), Some(second.runtime()));
}

#[test]
fn invalid_or_duplicate_records_are_rejected_as_a_complete_document() {
    let directory = TempDir::new().unwrap();
    let store = store(&directory);
    fs::write(
        store.path(),
        serde_json::to_vec(&json!({
            "formatVersion": 1,
            "connections": [
                record("/runtime/one/bin/zeta"),
                record("/runtime/two/bin/zeta")
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let error = store.connections().unwrap_err();
    assert_eq!(
        error.kind(),
        RemoteConnectionProfileStoreFailureKind::Invalid
    );
    assert!(error.to_string().contains("repeats"));

    fs::write(
        store.path(),
        br#"{"formatVersion":1,"connections":[],"secret":"x"}"#,
    )
    .unwrap();
    let error = store.connections().unwrap_err();
    assert_eq!(
        error.kind(),
        RemoteConnectionProfileStoreFailureKind::Invalid
    );
}

#[test]
fn advisory_lease_prevents_a_lost_update_and_releases_on_drop() {
    let directory = TempDir::new().unwrap();
    let store = store(&directory);
    let first = profile(target(), "/runtime/one/bin/zeta");
    store.activate(&first).unwrap();
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .open(store.lock_path().unwrap())
        .unwrap();
    lease.try_lock().unwrap();

    let error = store
        .activate(&profile(target(), "/runtime/two/bin/zeta"))
        .unwrap_err();
    assert_eq!(error.kind(), RemoteConnectionProfileStoreFailureKind::Busy);
    drop(lease);
    assert_eq!(
        store
            .connection(&target())
            .unwrap()
            .unwrap()
            .active_runtime()
            .executable(),
        "/runtime/one/bin/zeta"
    );
}

#[cfg(unix)]
#[test]
fn profile_resource_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let directory = TempDir::new().unwrap();
    let store = store(&directory);
    let outside = directory.path().join("outside.json");
    fs::write(&outside, r#"{"formatVersion":1,"connections":[]}"#).unwrap();
    symlink(outside, store.path()).unwrap();

    let error = store.connections().unwrap_err();
    assert_eq!(
        error.kind(),
        RemoteConnectionProfileStoreFailureKind::Invalid
    );
}

fn store(directory: &TempDir) -> RemoteConnectionProfileStore {
    RemoteConnectionProfileStore::new(directory.path().join("remote-connections.json"))
}

fn target() -> SshTarget {
    SshTarget::new(
        SshHost::parse("build.example").unwrap(),
        RemoteWorkspacePath::parse("/srv/project").unwrap(),
    )
}

fn profile(target: SshTarget, runtime: &str) -> RemoteProfile {
    RemoteProfile::new(target, RemoteRuntime::new(runtime).unwrap())
}

fn record(active_runtime: &str) -> serde_json::Value {
    json!({
        "host": "build.example",
        "workspace": "/srv/project",
        "activeRuntime": active_runtime,
    })
}
