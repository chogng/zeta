use super::*;
use pretty_assertions::assert_eq;
use std::path::PathBuf;

#[test]
fn identical_existing_paths_match_after_normalization() {
    let directory = tempfile::tempdir().expect("temporary directory");

    assert!(paths_match_after_normalization(
        directory.path(),
        directory.path()
    ));
}

#[test]
fn missing_paths_fall_back_to_direct_equality() {
    assert!(paths_match_after_normalization("missing", "missing"));
    assert!(!paths_match_after_normalization(
        "missing-left",
        "missing-right"
    ));
}

#[cfg(unix)]
#[test]
fn symlink_targets_compare_as_the_same_path() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().expect("temporary directory");
    let target = directory.path().join("target");
    let alias = directory.path().join("alias");
    std::fs::create_dir(&target).expect("target directory");
    symlink(&target, &alias).expect("symlink");

    assert!(paths_match_after_normalization(target, alias));
}

#[cfg(unix)]
#[test]
fn relative_symlink_write_target_is_resolved() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().expect("temporary directory");
    let target = directory.path().join("target");
    let alias = directory.path().join("alias");
    symlink("target", &alias).expect("relative symlink");

    assert_eq!(
        resolve_symlink_write_paths(&alias),
        SymlinkWritePaths {
            read_path: Some(target.clone()),
            write_path: target,
        }
    );
}

#[cfg(unix)]
#[test]
fn symlink_cycles_fall_back_to_the_original_write_path() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().expect("temporary directory");
    let first = directory.path().join("first");
    let second = directory.path().join("second");
    symlink(&second, &first).expect("first symlink");
    symlink(&first, &second).expect("second symlink");

    assert_eq!(
        resolve_symlink_write_paths(&first),
        SymlinkWritePaths {
            read_path: None,
            write_path: first,
        }
    );
}

#[test]
fn atomic_write_replaces_existing_contents() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("nested").join("state.json");
    write_text_atomically(&path, "first").expect("initial write");
    write_text_atomically(&path, "second").expect("replacement write");

    assert_eq!(std::fs::read_to_string(path).unwrap(), "second");
}

#[test]
fn native_workdir_is_unchanged_when_windows_rules_are_disabled() {
    let path = PathBuf::from(r"\\?\D:\worktree");

    assert_eq!(
        comparison::normalize_for_native_workdir_on(path.clone(), false),
        path
    );
}

#[cfg(target_os = "linux")]
#[test]
fn wsl_drive_mounts_are_ascii_lowercased() {
    assert_eq!(
        comparison::normalize_for_wsl_on(PathBuf::from("/mnt/C/Users/Dev"), true),
        PathBuf::from("/mnt/c/users/dev")
    );
    assert_eq!(
        comparison::normalize_for_wsl_on(PathBuf::from("/home/Dev"), true),
        PathBuf::from("/home/Dev")
    );
}
