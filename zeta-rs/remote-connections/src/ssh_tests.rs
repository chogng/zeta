#[cfg(unix)]
use std::fs;
use std::num::NonZeroU16;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::RemoteConnectionFailureKind;
use super::RemoteRuntimeProbe;
use super::SshAppServerConnectionOptions;
use super::remote_app_server_command;
use super::ssh::RuntimeProbeOutput;
use super::ssh::parse_runtime_probe_output;
#[cfg(unix)]
use zeta_app_server_protocol::protocol::common::ClientCapabilities;
#[cfg(unix)]
use zeta_app_server_protocol::protocol::common::ClientInfo;
#[cfg(unix)]
use zeta_app_server_protocol::protocol::initialize::{
    APP_SERVER_PROTOCOL_MAJOR, APP_SERVER_PROTOCOL_REVISION,
};
use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;

fn profile() -> RemoteProfile {
    RemoteProfile::new(
        SshTarget::new(
            SshHost::parse("build-linux").unwrap(),
            RemoteWorkspacePath::parse("/srv/zeta/project with spaces").unwrap(),
        ),
        RemoteRuntime::new("/opt/zeta/bin/zeta-remote-server").unwrap(),
    )
}

#[test]
fn ssh_connection_starts_a_non_interactive_stdio_channel() {
    let command = SshAppServerConnectionOptions::new(profile())
        .with_ssh_executable("/usr/bin/ssh")
        .with_connect_timeout_seconds(NonZeroU16::new(15).unwrap())
        .stdio_command();

    assert_eq!(command.executable(), Path::new("/usr/bin/ssh"));
    assert_eq!(
        command.arguments_as_strings(),
        vec![
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=15",
            "build-linux",
            "'env' 'ZETA_WORKSPACE_ROOT=/srv/zeta/project with spaces' '/opt/zeta/bin/zeta-remote-server' 'remote-server' 'connect'",
        ]
    );
}

#[test]
fn remote_command_quotes_the_workspace_and_runtime_as_independent_arguments() {
    let profile = RemoteProfile::new(
        SshTarget::new(
            SshHost::parse("build-linux").unwrap(),
            RemoteWorkspacePath::parse("/srv/o'reilly").unwrap(),
        ),
        RemoteRuntime::new("/opt/zeta remote/bin/server").unwrap(),
    );

    assert_eq!(
        remote_app_server_command(&profile),
        "'env' 'ZETA_WORKSPACE_ROOT=/srv/o'\\''reilly' '/opt/zeta remote/bin/server' 'remote-server' 'connect'"
    );
}

#[test]
fn remote_runtime_probe_is_shell_quoted_and_reports_a_resolved_executable() {
    let profile = RemoteProfile::new(
        SshTarget::new(
            SshHost::parse("build-linux").unwrap(),
            RemoteWorkspacePath::parse("/srv/zeta").unwrap(),
        ),
        RemoteRuntime::new("/opt/zeta's/bin/zeta-server").unwrap(),
    );
    let options = SshAppServerConnectionOptions::new(profile);

    assert_eq!(
        super::ssh::remote_runtime_probe_command(options.profile().runtime().executable()),
        "if command -v '/opt/zeta'\\''s/bin/zeta-server' >/dev/null 2>&1; then printf '%s%s\\n' '__ZETA_REMOTE_RUNTIME_FOUND__:' \"$(command -v '/opt/zeta'\\''s/bin/zeta-server')\"; else printf '%s\\n' '__ZETA_REMOTE_RUNTIME_MISSING__'; exit 127; fi"
    );

    let probe = RemoteRuntimeProbe {
        requested_runtime: RemoteRuntime::new("/opt/zeta/bin/zeta-server").unwrap(),
        resolved_runtime: RemoteRuntime::new("/opt/zeta/bin/zeta-server").unwrap(),
    };
    assert_eq!(probe.requested_executable(), "/opt/zeta/bin/zeta-server");
    assert_eq!(probe.resolved_executable(), "/opt/zeta/bin/zeta-server");
    assert_eq!(
        parse_runtime_probe_output(
            "login banner\n__ZETA_REMOTE_RUNTIME_FOUND__:/usr/bin/zeta-server\n"
        ),
        Some(RuntimeProbeOutput::Found("/usr/bin/zeta-server".into()))
    );
    assert_eq!(
        parse_runtime_probe_output("__ZETA_REMOTE_RUNTIME_MISSING__\n"),
        Some(RuntimeProbeOutput::Missing)
    );
    assert_eq!(parse_runtime_probe_output("unexpected output\n"), None);
}

#[test]
fn remote_connection_errors_keep_stable_failure_categories() {
    let transport = super::RemoteConnectionError::from_client_error(
        zeta_app_server_client::ClientError::Transport("ssh failed".into()),
    );
    let protocol = super::RemoteConnectionError::from_client_error(
        zeta_app_server_client::ClientError::Protocol("protocol major mismatch".into()),
    );
    let server = super::RemoteConnectionError::from_client_error(
        zeta_app_server_client::ClientError::Server {
            code: -32001,
            message: "unsupported".into(),
        },
    );

    assert_eq!(transport.kind(), RemoteConnectionFailureKind::Transport);
    assert_eq!(
        protocol.kind(),
        RemoteConnectionFailureKind::ProtocolIncompatible
    );
    assert_eq!(server.kind(), RemoteConnectionFailureKind::ServerRejected);
}

#[cfg(unix)]
#[test]
fn runtime_probe_distinguishes_available_and_missing_runtime() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("fake-ssh");
    fs::write(
        &executable,
        "#!/bin/sh\ncase \"$*\" in\n  *missing-runtime*) printf '%s\\n' '__ZETA_REMOTE_RUNTIME_MISSING__'; exit 127 ;;\n  *) printf '%s%s\\n' '__ZETA_REMOTE_RUNTIME_FOUND__:' '/usr/bin/zeta-server' ;;\nesac\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();

    let available = SshAppServerConnectionOptions::new(profile())
        .with_ssh_executable(&executable)
        .probe_runtime()
        .unwrap();
    assert_eq!(
        available.requested_executable(),
        "/opt/zeta/bin/zeta-remote-server"
    );
    assert_eq!(available.resolved_executable(), "/usr/bin/zeta-server");

    let missing_profile = RemoteProfile::new(
        SshTarget::new(
            SshHost::parse("build-linux").unwrap(),
            RemoteWorkspacePath::parse("/srv/zeta/project").unwrap(),
        ),
        RemoteRuntime::new("missing-runtime").unwrap(),
    );
    let error = SshAppServerConnectionOptions::new(missing_profile)
        .with_ssh_executable(executable)
        .probe_runtime()
        .unwrap_err();
    assert_eq!(
        error.kind(),
        RemoteConnectionFailureKind::RuntimeUnavailable
    );
}

#[cfg(unix)]
#[test]
fn compatibility_probe_uses_protocol_capabilities_and_tolerates_schema_drift() {
    let directory = tempfile::tempdir().unwrap();
    let compatible = directory.path().join("compatible-ssh");
    write_initialize_server(&compatible, APP_SERVER_PROTOCOL_MAJOR, "newer-schema");

    let initialization = SshAppServerConnectionOptions::new(profile())
        .with_ssh_executable(&compatible)
        .probe_compatibility(client_info(), ClientCapabilities::default())
        .unwrap();
    assert_eq!(initialization.schema_hash.0, "newer-schema");

    let incompatible = directory.path().join("incompatible-ssh");
    write_initialize_server(&incompatible, APP_SERVER_PROTOCOL_MAJOR + 1, "newer-schema");
    let error = SshAppServerConnectionOptions::new(profile())
        .with_ssh_executable(incompatible)
        .probe_compatibility(client_info(), ClientCapabilities::default())
        .unwrap_err();
    assert_eq!(
        error.kind(),
        RemoteConnectionFailureKind::ProtocolIncompatible
    );
}

#[cfg(unix)]
fn client_info() -> ClientInfo {
    ClientInfo {
        name: "remote-connection-test".into(),
        version: "1".into(),
    }
}

#[cfg(unix)]
fn write_initialize_server(path: &Path, protocol_major: u32, server_schema_hash: &str) {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "serverInfo": { "name": "fake-remote", "version": "1" },
            "protocolVersion": {
                "major": protocol_major,
                "revision": APP_SERVER_PROTOCOL_REVISION
            },
            "schemaHash": server_schema_hash,
            "capabilities": {
                "agentInteractions": false,
                "documentCollaboration": false,
                "sessions": true,
                "threads": true,
                "turns": true,
                "resources": false,
                "attachments": false,
                "fileSystem": false,
                "git": false,
                "workspaceSearch": false,
                "codebase": false,
                "cloudCodebase": false,
                "terminal": false,
                "debugAdapter": false,
                "typst": false,
                "updateReplay": false,
                "extensions": false,
                "extensionHost": false,
                "connectors": false,
                "plugins": false,
                "marketplace": false,
                "mcp": false,
                "mcpOAuth": false,
                "contracts": {
                    "sessions": { "version": 1 },
                    "threads": { "version": 1 },
                    "turns": { "version": 1 }
                }
            },
            "slashCommands": []
        }
    })
    .to_string();
    fs::write(
        path,
        format!("#!/bin/sh\nIFS= read -r request || exit 65\nprintf '%s\\n' '{response}'\n"),
    )
    .unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}
