use std::path::Path;
use std::path::PathBuf;

use super::AppServerHost;
use super::local_app_server_command;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_daemon::DAEMON_PATH_ENV;
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;

#[test]
fn ssh_app_server_host_retargets_the_same_backend_to_another_workspace() {
    let host = AppServerHost::remote_with_executable(
        RemoteProfile::new(
            SshTarget::new(
                SshHost::parse("build-linux").unwrap(),
                RemoteWorkspacePath::parse("/srv/zeta").unwrap(),
            ),
            RemoteRuntime::new("zeta-remote-server").unwrap(),
        ),
        None,
    );

    assert_eq!(host.workspace_root(), Path::new("/srv/zeta"));
    let (ssh_host, ssh_executable) = host.ssh_transport().unwrap();
    assert_eq!(ssh_host.as_str(), "build-linux");
    assert_eq!(ssh_executable, Path::new("ssh"));
    let switched = host.with_workspace_root(Path::new("/srv/other")).unwrap();
    assert_eq!(switched.workspace_root(), Path::new("/srv/other"));
    let (switched_host, _) = switched.ssh_transport().unwrap();
    assert_eq!(switched_host.as_str(), "build-linux");
}

#[test]
fn local_app_server_host_connects_through_the_profile_workspace_broker() {
    let executable = PathBuf::from("/opt/zeta/zeterm");
    let profile = PathBuf::from("/profiles/zeta");
    let workspace = PathBuf::from("/workspaces/project");
    let command = local_app_server_command(
        executable.clone(),
        profile.clone(),
        &workspace,
        Some(executable.clone()),
    );

    assert_eq!(command.executable(), executable);
    assert_eq!(command.arguments_as_strings(), ["app-server", "connect"]);
    assert_eq!(
        command,
        StdioAppServerCommand::new("/opt/zeta/zeterm")
            .with_argument("app-server")
            .with_argument("connect")
            .with_environment_variable("ZETA_PROFILE_ROOT", profile.into_os_string())
            .with_environment_variable("ZETA_WORKSPACE_ROOT", "/workspaces/project")
            .with_environment_variable(DAEMON_PATH_ENV, "/opt/zeta/zeterm"),
    );
    assert_eq!(
        AppServerHost::local(workspace)
            .with_workspace_root(Path::new("/workspaces/other"))
            .unwrap()
            .workspace_root(),
        Path::new("/workspaces/other"),
    );
}
