use std::path::Path;

use crate::agent_session_target::AgentSessionTarget;
use crate::agent_session_target::WorkspaceSwitchSupport;
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;

#[test]
fn ssh_agent_target_uses_the_remote_workspace_and_rejects_local_workspace_switches() {
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
    assert_eq!(
        target.workspace_switch_support(),
        WorkspaceSwitchSupport::Unsupported
    );
}
