use super::*;

#[test]
fn installation_requires_approval_of_the_current_complete_plan() {
    let one = setup_plan(1).unwrap();
    let two = setup_plan(2).unwrap();
    assert_ne!(plan_digest(&one).unwrap(), plan_digest(&two).unwrap());
    assert!(
        setup(1, &plan_digest(&two).unwrap())
            .unwrap_err()
            .contains("approval does not match")
    );
    assert!(
        setup(1, "")
            .unwrap_err()
            .contains("approval does not match")
    );
    assert_eq!(one["accounts"]["total"], 3);
    assert_eq!(one["network"]["persistentFilters"], 13);
    assert!(one.get("deviceAclChanges").is_none());
}

fn layout() -> (tempfile::TempDir, String) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("bin")).unwrap();
    std::fs::create_dir(temp.path().join("runs")).unwrap();
    std::fs::write(
        temp.path().join("bin/ash-windows-sandbox.exe"),
        b"owned runner",
    )
    .unwrap();
    for name in [
        "setup.lock",
        "state.dpapi",
        "state.pending",
        "lease-0",
        "lease-1",
    ] {
        std::fs::write(temp.path().join(name), b"test data").unwrap();
    }
    let digest = hash(&temp.path().join("bin/ash-windows-sandbox.exe")).unwrap();
    (temp, digest)
}

#[test]
fn cleanup_removes_owned_files_and_keeps_the_journal_until_final_verification() {
    let (temp, digest) = layout();
    clean_files(temp.path(), &digest, 2).unwrap();
    let mut remaining = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    remaining.sort();
    assert_eq!(remaining, ["setup.lock", "state.dpapi"]);
    clean_files(temp.path(), &digest, 2).unwrap();
}

#[test]
fn unknown_files_or_incomplete_execution_preserve_recovery_state() {
    for entry in ["unexpected", "runs/incomplete"] {
        let (temp, digest) = layout();
        std::fs::write(temp.path().join(entry), b"preserve me").unwrap();
        assert!(clean_files(temp.path(), &digest, 2).is_err());
        assert_eq!(
            std::fs::read(temp.path().join(entry)).unwrap(),
            b"preserve me"
        );
        assert!(temp.path().join("state.dpapi").exists());
    }
}

#[test]
fn a_changed_runtime_is_not_silently_deleted() {
    let (temp, digest) = layout();
    std::fs::write(
        temp.path().join("bin/ash-windows-sandbox.exe"),
        b"different file",
    )
    .unwrap();
    assert!(clean_files(temp.path(), &digest, 2).is_err());
    assert!(temp.path().join("bin/ash-windows-sandbox.exe").exists());
    assert!(temp.path().join("state.dpapi").exists());
}
