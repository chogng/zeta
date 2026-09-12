use std::process::Command;
use tempfile::tempdir;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
use zeta_app_server_protocol::protocol::common::ClientInfo;

#[test]
fn renamed_daemon_commands_launch_and_stop_the_same_generation() {
    let root = tempdir().unwrap();
    let executable = root.path().join(if cfg!(windows) {
        "zeta-app-server-daemon.generation.exe"
    } else {
        "zeta-app-server-daemon.generation"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_zeta-app-server-daemon"), &executable).unwrap();
    let profile = root.path().join("profile");
    let session = AppServerSession::start_stdio(
        StdioAppServerCommand::new(&executable)
            .with_argument("connect")
            .with_environment_variable("ZETA_PROFILE_ROOT", profile.clone().into_os_string())
            .without_environment_variable("ZETA_WORKSPACE_ROOT"),
        ClientInfo {
            name: "generation-command-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let result = session.client().list_sessions();
    session.shutdown().unwrap();
    let stopped = Command::new(&executable)
        .arg("stop")
        .env("ZETA_PROFILE_ROOT", &profile)
        .output()
        .unwrap();
    assert!(
        stopped.status.success(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    assert!(result.unwrap().sessions.is_empty());
}
