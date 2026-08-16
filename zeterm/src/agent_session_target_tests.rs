use std::path::Path;
use std::path::PathBuf;

use crate::agent_session_target::AgentSessionTarget;
use crate::agent_session_target::local_app_server_command;
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;

#[test]
fn ssh_agent_target_retargets_the_same_host_and_runtime_to_another_workspace() {
    let target = AgentSessionTarget::ssh_with_executable(
        RemoteProfile::new(
            SshTarget::new(
                SshHost::parse("build-linux").unwrap(),
                RemoteWorkspacePath::parse("/srv/zeta").unwrap(),
            ),
            RemoteRuntime::new("zeta-remote-server").unwrap(),
        ),
        None,
    );

    assert_eq!(target.workspace_root(), Path::new("/srv/zeta"));
    let (host, ssh_executable) = target.ssh_transport().unwrap();
    assert_eq!(host.as_str(), "build-linux");
    assert_eq!(ssh_executable, Path::new("ssh"));
    let switched = target.with_workspace_root(Path::new("/srv/other")).unwrap();
    assert_eq!(switched.workspace_root(), Path::new("/srv/other"));
    let (switched_host, _) = switched.ssh_transport().unwrap();
    assert_eq!(switched_host.as_str(), "build-linux");
}

#[test]
fn local_agent_target_connects_through_the_profile_workspace_broker() {
    let executable = PathBuf::from("/opt/zeta/zeterm");
    let profile = PathBuf::from("/profiles/zeta");
    let workspace = PathBuf::from("/workspaces/project");
    let command = local_app_server_command(executable.clone(), profile, &workspace);

    assert_eq!(command.executable(), executable);
    assert_eq!(command.arguments_as_strings(), ["app-server", "connect"]);
    assert_eq!(
        AgentSessionTarget::local(workspace)
            .with_workspace_root(Path::new("/workspaces/other"))
            .unwrap()
            .workspace_root(),
        Path::new("/workspaces/other"),
    );
}
