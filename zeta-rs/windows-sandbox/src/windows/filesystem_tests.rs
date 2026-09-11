use super::*;

#[test]
fn a_write_grant_cannot_change_an_outside_file_through_a_hard_link() {
    let temp = tempfile::tempdir().unwrap();
    let work = temp.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let outside = temp.path().join("outside");
    std::fs::write(&outside, "unchanged").unwrap();
    std::fs::hard_link(&outside, work.join("alias")).unwrap();
    let mut policy = wxc_common::models::ContainerPolicy::default();
    policy.readwrite_paths.push(work.to_str().unwrap().into());
    assert!(validate(&policy).unwrap_err().contains("multiply linked"));
    assert_eq!(std::fs::read_to_string(outside).unwrap(), "unchanged");
}

#[test]
fn separate_acl_journals_restore_only_their_own_mutations() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    let mut one = DaclManager::in_directory(&temp.path().join("journal-one")).unwrap();
    let mut two = DaclManager::in_directory(&temp.path().join("journal-two")).unwrap();
    one.deny_write_access("S-1-5-21-911-912-913-914", &[first])
        .unwrap();
    two.deny_write_access("S-1-5-21-921-922-923-924", &[second])
        .unwrap();
    one.restore_strict().unwrap();
    assert_eq!(
        std::fs::read_dir(temp.path().join("journal-one"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        std::fs::read_dir(temp.path().join("journal-two"))
            .unwrap()
            .count(),
        1
    );
    two.restore_strict().unwrap();
    assert_eq!(
        std::fs::read_dir(temp.path().join("journal-two"))
            .unwrap()
            .count(),
        0
    );
}
