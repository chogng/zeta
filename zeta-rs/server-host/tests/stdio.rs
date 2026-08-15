use tempfile::tempdir;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
use zeta_app_server_protocol::protocol::common::ClientInfo;

#[test]
fn server_host_serves_an_explicit_workspace_over_stdio() {
    let root = tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let profile = root.path().join("profile");
    std::fs::create_dir(&workspace).unwrap();
    let command = StdioAppServerCommand::new(env!("CARGO_BIN_EXE_zeta-server"))
        .with_argument("app-server")
        .with_argument("--listen")
        .with_argument("stdio://")
        .with_environment_variable("ZETA_WORKSPACE_ROOT", workspace.into_os_string())
        .with_environment_variable("ZETA_PROFILE_ROOT", profile.into_os_string());
    let session = AppServerSession::start_stdio(
        command,
        ClientInfo {
            name: "zeta-server-host-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let mut client = session.client();

    assert!(client.list_sessions().unwrap().sessions.is_empty());
    session.shutdown().unwrap();
}

#[test]
fn server_host_without_workspace_does_not_inherit_its_current_directory() {
    let root = tempdir().unwrap();
    let profile = root.path().join("profile");
    let command = StdioAppServerCommand::new(env!("CARGO_BIN_EXE_zeta-server"))
        .with_argument("app-server")
        .with_argument("--listen")
        .with_argument("stdio://")
        .without_environment_variable("ZETA_WORKSPACE_ROOT")
        .with_environment_variable("ZETA_PROFILE_ROOT", profile.into_os_string());
    let session = AppServerSession::start_stdio(
        command,
        ClientInfo {
            name: "zeta-server-host-empty-workspace-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let mut client = session.client();

    assert_eq!(
        client.git_status().unwrap_err(),
        ClientError::Server {
            code: -32060,
            message: "GitUnavailable".into(),
        }
    );
    session.shutdown().unwrap();
}
