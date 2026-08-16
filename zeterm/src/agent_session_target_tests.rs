use std::path::Path;

use crate::agent_session_target::AgentSessionTarget;
use crate::agent_session_target::WorkspaceSwitchSupport;
use crate::agent_session_target::local_client_options;
use zeta_app_server_client::SessionStateMode;
use zeta_app_server_client::start_in_process_client;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::session::SessionCreateParams;
use zeta_app_server_protocol::protocol::session::SessionRequest;
use zeta_app_server_protocol::protocol::session::SessionRequestParams;
use zeta_app_server_protocol::protocol::session::SessionRequestResult;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_protocol::CommandId;
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

#[test]
fn local_agent_target_recovers_sessions_from_the_shared_profile() {
    let profile = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let options = local_client_options(
        profile.path().to_path_buf(),
        workspace.path(),
        ClientInfo {
            name: "zeterm-test".into(),
            version: "1".into(),
        },
    );
    assert_eq!(options.session_state_mode, SessionStateMode::Durable);

    let mut first = start_in_process_client(options).unwrap();
    let created = first
        .create_session(SessionCreateParams {
            command_id: CommandId::new("create-shared-session").unwrap(),
            title: "Shared conversation".into(),
        })
        .unwrap();
    let thread = first
        .request_session(SessionRequestParams {
            command_id: CommandId::new("create-shared-thread").unwrap(),
            session_id: created.session.session_id.clone(),
            expected_sequence: created.session.sequence,
            request: SessionRequest::CreateThread {
                title: "Main".into(),
            },
        })
        .unwrap();
    let SessionRequestResult::Thread(thread) = thread else {
        panic!("createThread returned a non-Thread result");
    };
    drop(first);

    let mut reopened = start_in_process_client(local_client_options(
        profile.path().to_path_buf(),
        workspace.path(),
        ClientInfo {
            name: "zeterm-reopened-test".into(),
            version: "1".into(),
        },
    ))
    .unwrap();
    let sessions = reopened.list_sessions().unwrap().sessions;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].title, "Shared conversation");
    assert_eq!(sessions[0].threads.len(), 1);
    let recovered = reopened
        .read_session_thread(SessionThreadReadParams {
            session_id: sessions[0].session_id.clone(),
            thread_id: thread.thread_id.clone(),
            history: None,
        })
        .unwrap();
    assert_eq!(recovered.thread.thread_id, thread.thread_id);
}
