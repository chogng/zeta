#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use zeta_app_server_client::AppServerEvent;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::ConnectionCloseReason;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::terminal::TerminalAttachParams;
use zeta_app_server_protocol::protocol::terminal::TerminalCloseParams;
use zeta_app_server_protocol::protocol::terminal::TerminalCreateParams;
use zeta_app_server_protocol::protocol::terminal::TerminalLifecycle;
use zeta_app_server_protocol::protocol::terminal::TerminalProfileSelection;
#[cfg(unix)]
use zeta_remote::RemoteProfile;
#[cfg(unix)]
use zeta_remote::RemoteRuntime;
#[cfg(unix)]
use zeta_remote::RemoteWorkspacePath;
#[cfg(unix)]
use zeta_remote::SshHost;
#[cfg(unix)]
use zeta_remote::SshTarget;
#[cfg(unix)]
use zeta_remote_connections::SshAppServerConnectionOptions;

#[test]
fn zeta_code_cli_serves_the_remote_stdio_contract() {
    let root = test_root("stdio");
    let workspace = root.join("workspace");
    let profile = root.join("profile");
    std::fs::create_dir_all(&workspace).unwrap();

    let command = StdioAppServerCommand::new(env!("CARGO_BIN_EXE_zeta"))
        .with_argument("app-server")
        .with_argument("--listen")
        .with_argument("stdio://")
        .with_environment_variable("ZETA_WORKSPACE_ROOT", workspace.into_os_string())
        .with_environment_variable("ZETA_PROFILE_ROOT", profile.into_os_string());
    let mut session = AppServerSession::start_stdio(
        command,
        ClientInfo {
            name: "zeta-code-remote-test".into(),
            version: "1".into(),
        },
        ClientCapabilities::default(),
    )
    .unwrap();
    let events = session.take_events().unwrap();
    let mut client = session.client();

    assert!(client.list_sessions().unwrap().sessions.is_empty());

    session.shutdown().unwrap();
    assert_eq!(
        events
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap(),
        AppServerEvent::ConnectionClosed(ConnectionCloseReason::Shutdown)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn zeta_code_remote_server_preserves_a_terminal_between_real_cli_connections() {
    let root = test_root("broker");
    let workspace = root.join("workspace");
    let profile = root.join("profile");
    std::fs::create_dir_all(&workspace).unwrap();
    let command = || {
        StdioAppServerCommand::new(env!("CARGO_BIN_EXE_zeta"))
            .with_argument("remote-server")
            .with_argument("connect")
            .with_environment_variable("ZETA_WORKSPACE_ROOT", workspace.clone().into_os_string())
            .with_environment_variable("ZETA_PROFILE_ROOT", profile.clone().into_os_string())
            .with_environment_variable("ZETA_REMOTE_SERVER_IDLE_TIMEOUT_MILLIS", "200")
    };
    let client_info = || ClientInfo {
        name: "zeta-code-remote-broker-test".into(),
        version: "1".into(),
    };

    let first_session =
        AppServerSession::start_stdio(command(), client_info(), ClientCapabilities::default())
            .unwrap();
    let mut first_client = first_session.client();
    let created = first_client
        .terminal_create(TerminalCreateParams {
            rows: 24,
            cols: 80,
            profile: TerminalProfileSelection::Default,
            lifecycle: TerminalLifecycle::Reconnectable,
        })
        .unwrap();
    let first_lease = created.reconnect.unwrap();
    first_session.shutdown().unwrap();

    let second_session =
        AppServerSession::start_stdio(command(), client_info(), ClientCapabilities::default())
            .unwrap();
    let mut second_client = second_session.client();
    let deadline = Instant::now() + Duration::from_secs(2);
    let attached = loop {
        match second_client.terminal_attach(TerminalAttachParams {
            terminal_id: created.terminal_id.clone(),
            reconnect_token: first_lease.reconnect_token.clone(),
            rows: 30,
            cols: 100,
        }) {
            Ok(attached) => break attached,
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            Err(error) => panic!("terminal did not survive the CLI connection: {error}"),
        }
    };

    assert_ne!(
        attached.reconnect.reconnect_token,
        first_lease.reconnect_token
    );
    second_client
        .terminal_close(TerminalCloseParams {
            terminal_id: created.terminal_id,
        })
        .unwrap();
    second_session.shutdown().unwrap();
    thread::sleep(Duration::from_millis(500));
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn shared_ssh_options_reach_the_real_zeta_remote_server_entrypoint() {
    let root = test_root("ssh-transport");
    let workspace = root.join("workspace");
    let profile_root = root.join("profile");
    let fake_ssh = root.join("fake-ssh");
    fs::create_dir_all(&workspace).unwrap();
    assert!(!profile_root.to_string_lossy().contains('\''));
    fs::write(
        &fake_ssh,
        format!(
            "#!/bin/sh\nexport ZETA_PROFILE_ROOT='{}'\ncommand=''\nfor argument in \"$@\"; do command=$argument; done\nexec /bin/sh -c \"$command\"\n",
            profile_root.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&fake_ssh).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_ssh, permissions).unwrap();
    let remote_profile = RemoteProfile::new(
        SshTarget::new(
            SshHost::parse("local-ssh-double").unwrap(),
            RemoteWorkspacePath::parse(workspace.to_str().unwrap()).unwrap(),
        ),
        RemoteRuntime::new_exact_executable(env!("CARGO_BIN_EXE_zeta")).unwrap(),
    );
    let connection =
        SshAppServerConnectionOptions::new(remote_profile).with_ssh_executable(&fake_ssh);

    let session = connection
        .connect(
            ClientInfo {
                name: "zeta-code-local-ssh-test".into(),
                version: "1".into(),
            },
            ClientCapabilities::default(),
        )
        .unwrap();
    let mut client = session.client();

    assert!(client.list_sessions().unwrap().sessions.is_empty());

    session.shutdown().unwrap();
    thread::sleep(Duration::from_millis(500));
    fs::remove_dir_all(root).unwrap();
}

fn test_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-cli-remote-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ))
}
