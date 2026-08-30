use std::path::Path;
use std::path::PathBuf;

use super::AppServerHost;
use super::local_app_server_command;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_daemon::DAEMON_PATH_ENV;
use zeta_remote::RemoteDirPath;
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;

#[test]
fn ssh_app_server_host_retargets_the_same_backend_to_another_dir() {
    let host = AppServerHost::remote_with_executable(
        RemoteProfile::new(
            SshTarget::new(
                SshHost::parse("build-linux").unwrap(),
                RemoteDirPath::parse("/srv/zeta").unwrap(),
            ),
            RemoteRuntime::new("zeta-remote-server").unwrap(),
        ),
        None,
    );

    assert_eq!(host.cwd(), Path::new("/srv/zeta"));
    let (ssh_host, ssh_executable) = host.ssh_transport().unwrap();
    assert_eq!(ssh_host.as_str(), "build-linux");
    assert_eq!(ssh_executable, Path::new("ssh"));
    let switched = host.with_cwd(Path::new("/srv/other")).unwrap();
    assert_eq!(switched.cwd(), Path::new("/srv/other"));
    let (switched_host, _) = switched.ssh_transport().unwrap();
    assert_eq!(switched_host.as_str(), "build-linux");
}

#[test]
fn local_app_server_host_connects_through_the_profile_dir_broker() {
    let executable = PathBuf::from("/opt/zeta/app");
    let profile = PathBuf::from("/profiles/zeta");
    let dir = PathBuf::from("/dirs/project");
    let command = local_app_server_command(
        executable.clone(),
        profile.clone(),
        &dir,
        Some(executable.clone()),
    );

    assert_eq!(command.executable(), executable);
    assert_eq!(command.arguments_as_strings(), ["app-server", "connect"]);
    assert_eq!(
        command,
        StdioAppServerCommand::new("/opt/zeta/app")
            .with_argument("app-server")
            .with_argument("connect")
            .with_environment_variable("ZETA_PROFILE_ROOT", profile.into_os_string())
            .with_environment_variable("ZETA_WORKSPACE_ROOT", "/dirs/project")
            .with_environment_variable(DAEMON_PATH_ENV, "/opt/zeta/app"),
    );
    assert_eq!(
        AppServerHost::local(dir)
            .with_cwd(Path::new("/dirs/other"))
            .unwrap()
            .cwd(),
        Path::new("/dirs/other"),
    );
}
