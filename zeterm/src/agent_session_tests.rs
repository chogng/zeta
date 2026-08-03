use super::{git_is_unavailable, workspace_title};
use std::path::Path;
use zeta_app_server_client::ClientError;

#[test]
fn workspace_title_uses_the_last_path_component() {
    assert_eq!(workspace_title(Path::new("/work/zeta")), "zeta");
}

#[test]
fn workspace_title_has_a_stable_root_fallback() {
    assert_eq!(workspace_title(Path::new("/")), "Agent Session");
}

#[test]
fn git_unavailable_does_not_hide_operation_failures() {
    let server_error = |code| ClientError::Server {
        code,
        message: "Git error".into(),
    };

    assert!(git_is_unavailable(&server_error(-32060)));
    assert!(git_is_unavailable(&server_error(-32062)));
    assert!(!git_is_unavailable(&server_error(-32061)));
}
