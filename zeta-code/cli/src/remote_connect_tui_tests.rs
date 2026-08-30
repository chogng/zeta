use std::path::Path;

use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_remote::RemoteDirPath;
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;

use super::recovery_command;

#[test]
fn remote_recovery_command_preserves_the_verified_connection() {
    let profile = RemoteProfile::new(
        SshTarget::new(
            SshHost::parse("build-linux").unwrap(),
            RemoteDirPath::parse("/srv/project with spaces").unwrap(),
        ),
        RemoteRuntime::new_exact_executable("/srv/zeta/runtime/bin/zeta-server").unwrap(),
    );
    let recovery = zeta_tui::TuiRecoveryState::new(
        SessionId::new("session-1").unwrap(),
        ThreadId::new("thread-1").unwrap(),
    );

    assert_eq!(
        recovery_command(&profile, Some(Path::new("/opt/ssh client")), &recovery),
        [
            "zeta",
            "remote",
            "connect",
            "--host",
            "build-linux",
            "--dir",
            "/srv/project with spaces",
            "--runtime",
            "/srv/zeta/runtime/bin/zeta-server",
            "--ssh",
            "/opt/ssh client",
            "--resume",
            "session-1",
            "thread-1",
        ]
    );
}
