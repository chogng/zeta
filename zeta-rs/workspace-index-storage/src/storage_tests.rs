use std::fs;
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;

use tempfile::TempDir;
use zeta_workspace::WorkspaceTrustId;

use super::{ClearOutcome, WorkspaceIndexKind, WorkspaceIndexStorage};

fn workspace_id(byte: char) -> WorkspaceTrustId {
    WorkspaceTrustId::from_str(&format!("sha256:{}", byte.to_string().repeat(64))).unwrap()
}

#[test]
fn uses_one_external_directory_per_workspace() {
    let profile = TempDir::new().unwrap();
    let storage = WorkspaceIndexStorage::open(profile.path()).unwrap();
    let workspace = workspace_id('a');

    let lease = storage
        .acquire(&workspace, WorkspaceIndexKind::AgentGrep)
        .unwrap();

    assert_eq!(
        lease.directory(),
        profile
            .path()
            .join("cache/workspaces")
            .join("a".repeat(64))
            .join("indexes/agent-grep")
    );
    assert!(profile.path().join("cache/locks/indexes.lock").is_file());
    assert!(
        profile
            .path()
            .join("cache/locks/workspaces")
            .join("a".repeat(64))
            .join("agent-grep.lock")
            .is_file()
    );
}

#[test]
fn clear_index_waits_for_users_and_is_idempotent() {
    let profile = TempDir::new().unwrap();
    let storage = WorkspaceIndexStorage::open(profile.path()).unwrap();
    let workspace = workspace_id('b');
    let lease = storage
        .acquire(&workspace, WorkspaceIndexKind::Lexical)
        .unwrap();
    fs::write(lease.directory().join("index.sqlite3"), b"index").unwrap();

    assert_eq!(
        storage
            .clear_index(&workspace, WorkspaceIndexKind::Lexical)
            .unwrap(),
        ClearOutcome::InUse
    );
    drop(lease);
    assert_eq!(
        storage
            .clear_index(&workspace, WorkspaceIndexKind::Lexical)
            .unwrap(),
        ClearOutcome::Cleared
    );
    assert_eq!(
        storage
            .clear_index(&workspace, WorkspaceIndexKind::Lexical)
            .unwrap(),
        ClearOutcome::AlreadyAbsent
    );
}

#[test]
fn clear_workspace_is_atomic_across_index_kinds() {
    let profile = TempDir::new().unwrap();
    let storage = WorkspaceIndexStorage::open(profile.path()).unwrap();
    let workspace = workspace_id('c');
    let lexical = storage
        .acquire(&workspace, WorkspaceIndexKind::Lexical)
        .unwrap();
    let symbols = storage
        .acquire(&workspace, WorkspaceIndexKind::Symbols)
        .unwrap();
    fs::write(lexical.directory().join("index.sqlite3"), b"lexical").unwrap();
    fs::write(symbols.directory().join("index.sqlite3"), b"symbols").unwrap();

    assert_eq!(
        storage.clear_workspace(&workspace).unwrap(),
        ClearOutcome::InUse
    );
    drop(lexical);
    drop(symbols);
    assert_eq!(
        storage.clear_workspace(&workspace).unwrap(),
        ClearOutcome::Cleared
    );
}

#[test]
fn clear_all_does_not_remove_an_open_workspace() {
    let profile = TempDir::new().unwrap();
    let storage = WorkspaceIndexStorage::open(profile.path()).unwrap();
    let workspace = workspace_id('d');
    let lease = storage
        .acquire(&workspace, WorkspaceIndexKind::Semantic)
        .unwrap();

    assert_eq!(storage.clear_all().unwrap(), ClearOutcome::InUse);
    drop(lease);
    assert_eq!(storage.clear_all().unwrap(), ClearOutcome::Cleared);
    assert!(profile.path().join("cache/workspaces").is_dir());
}

#[test]
fn a_cross_process_lease_blocks_explicit_deletion() {
    let profile = TempDir::new().unwrap();
    let coordination = TempDir::new().unwrap();
    let ready = coordination.path().join("ready");
    let release = coordination.path().join("release");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "tests::cross_process_lease_child", "--nocapture"])
        .env("ZETA_INDEX_LOCK_TEST_ROOT", profile.path())
        .env("ZETA_INDEX_LOCK_TEST_READY", &ready)
        .env("ZETA_INDEX_LOCK_TEST_RELEASE", &release)
        .spawn()
        .unwrap();
    wait_for_path(&ready);

    let storage = WorkspaceIndexStorage::open(profile.path()).unwrap();
    assert_eq!(
        storage
            .clear_index(&workspace_id('f'), WorkspaceIndexKind::AgentGrep)
            .unwrap(),
        ClearOutcome::InUse
    );

    fs::write(&release, []).unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(
        storage
            .clear_index(&workspace_id('f'), WorkspaceIndexKind::AgentGrep)
            .unwrap(),
        ClearOutcome::Cleared
    );
}

#[test]
fn cross_process_lease_child() {
    let Some(profile) = std::env::var_os("ZETA_INDEX_LOCK_TEST_ROOT") else {
        return;
    };
    let storage = WorkspaceIndexStorage::open(profile).unwrap();
    let _lease = storage
        .acquire(&workspace_id('f'), WorkspaceIndexKind::AgentGrep)
        .unwrap();
    fs::write(std::env::var_os("ZETA_INDEX_LOCK_TEST_READY").unwrap(), []).unwrap();
    let release =
        std::path::PathBuf::from(std::env::var_os("ZETA_INDEX_LOCK_TEST_RELEASE").unwrap());
    wait_for_path(&release);
}

fn wait_for_path(path: &std::path::Path) {
    for _ in 0..500 {
        if path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {}", path.display());
}

#[cfg(unix)]
#[test]
fn refuses_to_follow_a_symlink_during_clear() {
    use std::os::unix::fs::symlink;

    let profile = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let storage = WorkspaceIndexStorage::open(profile.path()).unwrap();
    let workspace = workspace_id('e');
    let index_directory = storage.index_directory(&workspace, WorkspaceIndexKind::AgentGrep);
    fs::create_dir_all(index_directory.parent().unwrap()).unwrap();
    symlink(outside.path(), &index_directory).unwrap();

    let error = storage
        .clear_index(&workspace, WorkspaceIndexKind::AgentGrep)
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(outside.path().is_dir());
}
