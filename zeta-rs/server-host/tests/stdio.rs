use tempfile::tempdir;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::session::SessionCreateParams;
use zeta_protocol::CommandId;

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

#[test]
fn broker_shares_live_sessions_between_product_connections_and_workspaces() {
    let root = tempdir().unwrap();
    let first_workspace = root.path().join("first-workspace");
    let second_workspace = root.path().join("second-workspace");
    let profile = root.path().join("profile");
    std::fs::create_dir(&first_workspace).unwrap();
    std::fs::create_dir(&second_workspace).unwrap();
    let command = |workspace: &std::path::Path| {
        StdioAppServerCommand::new(env!("CARGO_BIN_EXE_zeta-server"))
            .with_argument("app-server")
            .with_argument("connect")
            .with_environment_variable("ZETA_WORKSPACE_ROOT", workspace.as_os_str())
            .with_environment_variable("ZETA_PROFILE_ROOT", profile.clone().into_os_string())
            .with_environment_variable("ZETA_LOCAL_APP_SERVER_IDLE_TIMEOUT_MILLIS", "50")
    };
    let first = AppServerSession::start_stdio(
        command(&first_workspace),
        ClientInfo {
            name: "zeta-desktop-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let second = AppServerSession::start_stdio(
        command(&second_workspace),
        ClientInfo {
            name: "zeterm-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let mut first_client = first.client();
    let mut second_client = second.client();

    assert!(second_client.list_sessions().unwrap().sessions.is_empty());
    first_client
        .create_session(SessionCreateParams {
            command_id: CommandId::new("create-cross-product-session").unwrap(),
            title: "Shared live conversation".into(),
        })
        .unwrap();
    let sessions = second_client.list_sessions().unwrap().sessions;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].title, "Shared live conversation");
    assert_eq!(
        sessions[0].workspace.as_ref().unwrap().root(),
        first_workspace.canonicalize().unwrap()
    );
    first.shutdown().unwrap();
    second.shutdown().unwrap();
}
