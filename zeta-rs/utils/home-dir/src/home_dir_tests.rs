use std::fs;
use std::io::ErrorKind;

use pretty_assertions::assert_eq;
use tempfile::TempDir;
use zeta_utils_absolute_path::AbsolutePathBuf;

use super::find_zeta_home_from;

#[test]
fn missing_explicit_profile_root_is_rejected() {
    let directory = TempDir::new().expect("temporary directory");
    let missing = directory.path().join("missing-profile");

    let error = find_zeta_home_from(Some(missing.as_os_str()), None)
        .expect_err("missing profile root must fail");

    assert_eq!(error.kind(), ErrorKind::NotFound);
    assert!(error.to_string().contains("ZETA_PROFILE_ROOT"));
}

#[test]
fn file_as_explicit_profile_root_is_rejected() {
    let directory = TempDir::new().expect("temporary directory");
    let file = directory.path().join("profile.txt");
    fs::write(&file, "not a directory").expect("write profile file");

    let error =
        find_zeta_home_from(Some(file.as_os_str()), None).expect_err("profile root file must fail");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert!(error.to_string().contains("not a directory"));
}

#[test]
fn explicit_profile_root_is_canonicalized() {
    let directory = TempDir::new().expect("temporary directory");
    fs::create_dir(directory.path().join("nested")).expect("nested directory");
    let configured = directory.path().join("nested").join("..");

    let resolved = find_zeta_home_from(Some(configured.as_os_str()), None)
        .expect("existing profile directory");
    let expected = AbsolutePathBuf::from_absolute(
        directory
            .path()
            .canonicalize()
            .expect("canonical profile root"),
    )
    .expect("absolute profile root");

    assert_eq!(resolved, expected);
}

#[test]
fn default_profile_root_does_not_need_to_exist() {
    let directory = TempDir::new().expect("temporary directory");
    let user_home = directory.path().join("missing-user-home");

    let resolved = find_zeta_home_from(None, Some(user_home.clone()))
        .expect("default profile root does not require filesystem access");

    assert_eq!(resolved.as_path(), user_home.join(".zeta"));
}

#[test]
fn missing_user_home_is_reported() {
    let error = find_zeta_home_from(None, None).expect_err("missing user home must fail");

    assert_eq!(error.kind(), ErrorKind::NotFound);
    assert!(error.to_string().contains("user home directory"));
}

#[test]
fn empty_profile_override_uses_default_home() {
    let directory = TempDir::new().expect("temporary directory");

    let resolved = find_zeta_home_from(
        Some(std::ffi::OsStr::new("")),
        Some(directory.path().to_path_buf()),
    )
    .expect("empty override uses default home");

    assert_eq!(resolved.as_path(), directory.path().join(".zeta"));
}
