use std::ffi::OsString;
use std::path::PathBuf;

use super::resolve_profile_root;

#[test]
fn explicit_profile_root_is_authoritative() {
    assert_eq!(
        resolve_profile_root(
            Some(OsString::from("/profiles/zeta")),
            Some(OsString::from("/home/ignored")),
        ),
        PathBuf::from("/profiles/zeta")
    );
}

#[test]
fn default_profile_root_is_shared_under_the_user_home() {
    assert_eq!(
        resolve_profile_root(None, Some(OsString::from("/home/ada"))),
        PathBuf::from("/home/ada/.zeta")
    );
}

#[test]
fn missing_home_uses_a_process_local_fallback() {
    assert_eq!(resolve_profile_root(None, None), PathBuf::from(".zeta"));
}
