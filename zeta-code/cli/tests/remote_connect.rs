#![cfg(unix)]

use std::fs;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use tar::Builder;
use tar::EntryType;
use tar::Header;

#[test]
fn resolves_a_saved_target_and_checks_the_real_broker() {
    let root = test_root("saved-target");
    let workspace = root.join("workspace");
    let profile_root = root.join("profile");
    let fake_ssh = root.join("fake-ssh");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        &fake_ssh,
        "#!/bin/sh\ncommand=''\nfor argument in \"$@\"; do command=$argument; done\nexec /bin/sh -c \"$command\"\n",
    )
    .unwrap();
    make_executable(&fake_ssh);

    let saved = Command::new(env!("CARGO_BIN_EXE_zeta"))
        .args([
            "remote",
            "connections",
            "save",
            "--name",
            "local-build",
            "--host",
            "local-ssh-double",
            "--workspace",
            workspace.to_str().unwrap(),
        ])
        .env("ZETA_PROFILE_ROOT", &profile_root)
        .output()
        .unwrap();
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );

    let connected = Command::new(env!("CARGO_BIN_EXE_zeta"))
        .args([
            "remote",
            "connect",
            "--name",
            "local-build",
            "--runtime",
            env!("CARGO_BIN_EXE_zeta"),
            "--ssh",
            fake_ssh.to_str().unwrap(),
            "--check",
        ])
        .env("ZETA_PROFILE_ROOT", &profile_root)
        .env("ZETA_REMOTE_SERVER_IDLE_TIMEOUT_MILLIS", "200")
        .output()
        .unwrap();
    assert!(
        connected.status.success(),
        "{}",
        String::from_utf8_lossy(&connected.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&connected.stdout).unwrap(),
        json!({
            "host": "local-ssh-double",
            "workspace": workspace.to_str().unwrap(),
            "activeRuntime": env!("CARGO_BIN_EXE_zeta"),
        })
    );
    assert!(profile_root.join("remote/connections.json").is_file());

    thread::sleep(Duration::from_millis(500));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn installs_a_missing_runtime_before_activating_it() {
    run_install_case(
        "install-missing",
        InitialRemoteRuntime::Missing,
        FailedCatalogAttempt::VerifyNoActivation,
    );
}

#[test]
fn replaces_a_schema_incompatible_managed_runtime() {
    run_install_case(
        "install-incompatible",
        InitialRemoteRuntime::Incompatible,
        FailedCatalogAttempt::Skip,
    );
}

#[derive(Clone, Copy)]
enum InitialRemoteRuntime {
    Missing,
    Incompatible,
}

#[derive(Clone, Copy)]
enum FailedCatalogAttempt {
    Skip,
    VerifyNoActivation,
}

fn run_install_case(
    label: &str,
    initial_runtime: InitialRemoteRuntime,
    failed_catalog_attempt: FailedCatalogAttempt,
) {
    let root = test_root(label);
    let workspace = root.join("workspace");
    let profile_root = root.join("profile");
    let artifacts = root.join("artifacts");
    let catalog_path = root.join("catalog.json");
    let fake_ssh = root.join("fake-ssh");
    let installed_state = root.join("installed");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&artifacts).unwrap();
    let artifact = create_runtime_archive(&artifacts);
    let catalog = json!({
        "formatVersion": 1,
        "artifacts": [{
            "version": "0.1.0",
            "target": "x86_64-unknown-linux-gnu",
            "archive": "artifacts/runtime.tar.gz",
            "archiveSize": artifact.archive_size,
            "unpackedSize": artifact.unpacked_size,
            "sha256": artifact.sha256,
        }],
    });
    let catalog_bytes = serde_json::to_vec(&catalog).unwrap();
    fs::write(&catalog_path, &catalog_bytes).unwrap();
    let catalog_sha256 = format!("{:x}", Sha256::digest(&catalog_bytes));
    write_installing_fake_ssh(
        &fake_ssh,
        &installed_state,
        &profile_root,
        &artifact.sha256,
        initial_runtime,
    );

    if matches!(
        failed_catalog_attempt,
        FailedCatalogAttempt::VerifyNoActivation
    ) {
        let rejected = connect_with_catalog(
            &workspace,
            &profile_root,
            &fake_ssh,
            &catalog_path,
            &"0".repeat(64),
        );
        assert!(!rejected.status.success());
        assert!(
            String::from_utf8_lossy(&rejected.stderr)
                .contains("Remote runtime catalog SHA-256 mismatch")
        );
        assert!(!profile_root.join("remote/connections.json").exists());
        assert!(!installed_state.exists());
    }

    let connected = connect_with_catalog(
        &workspace,
        &profile_root,
        &fake_ssh,
        &catalog_path,
        &catalog_sha256,
    );
    assert!(
        connected.status.success(),
        "{}",
        String::from_utf8_lossy(&connected.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&connected.stdout).unwrap(),
        json!({
            "host": "install-double",
            "workspace": workspace.to_str().unwrap(),
            "activeRuntime": env!("CARGO_BIN_EXE_zeta"),
        })
    );
    assert_eq!(fs::read_to_string(&installed_state).unwrap(), "install\n");

    let reconnected = Command::new(env!("CARGO_BIN_EXE_zeta"))
        .args([
            "remote",
            "connect",
            "--host",
            "install-double",
            "--workspace",
            workspace.to_str().unwrap(),
            "--ssh",
            fake_ssh.to_str().unwrap(),
            "--check",
        ])
        .env("ZETA_PROFILE_ROOT", &profile_root)
        .env("ZETA_REMOTE_SERVER_IDLE_TIMEOUT_MILLIS", "200")
        .output()
        .unwrap();
    assert!(
        reconnected.status.success(),
        "{}",
        String::from_utf8_lossy(&reconnected.stderr)
    );
    assert_eq!(fs::read_to_string(&installed_state).unwrap(), "install\n");

    thread::sleep(Duration::from_millis(500));
    fs::remove_dir_all(root).unwrap();
}

fn connect_with_catalog(
    workspace: &Path,
    profile_root: &Path,
    fake_ssh: &Path,
    catalog_path: &Path,
    catalog_sha256: &str,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_zeta"))
        .args([
            "remote",
            "connect",
            "--host",
            "install-double",
            "--workspace",
            workspace.to_str().unwrap(),
            "--ssh",
            fake_ssh.to_str().unwrap(),
            "--runtime-catalog",
            catalog_path.to_str().unwrap(),
            "--runtime-catalog-sha256",
            catalog_sha256,
            "--check",
        ])
        .env("ZETA_PROFILE_ROOT", profile_root)
        .env("ZETA_REMOTE_SERVER_IDLE_TIMEOUT_MILLIS", "200")
        .output()
        .unwrap()
}

fn write_installing_fake_ssh(
    path: &Path,
    installed_state: &Path,
    profile_root: &Path,
    artifact_sha256: &str,
    initial_runtime: InitialRemoteRuntime,
) {
    for value in [
        path,
        installed_state,
        profile_root,
        Path::new(env!("CARGO_BIN_EXE_zeta")),
    ] {
        assert!(!value.to_string_lossy().contains('\''));
    }
    let initial_probe = match initial_runtime {
        InitialRemoteRuntime::Missing => {
            "printf '%s\\n' '__ZETA_REMOTE_RUNTIME_MISSING__'; exit 127"
        }
        InitialRemoteRuntime::Incompatible => {
            "printf '%s\\n' '__ZETA_REMOTE_RUNTIME_FOUND__:/legacy/bin/zeta-server'"
        }
    };
    let obsolete_response = initialize_response("obsolete-schema");
    let receipt_runtime = format!(
        "/srv/zeta/remote/runtimes/x86_64-unknown-linux-gnu/0.1.0/{artifact_sha256}/bin/zeta-server"
    );
    fs::write(
        path,
        format!(
            "#!/bin/sh\ncommand=''\nfor argument in \"$@\"; do command=$argument; done\ncase \"$command\" in\n  *\"'remote-server' 'connect'\"*) if [ -f '{installed}' ]; then export ZETA_PROFILE_ROOT='{profile}'; exec /bin/sh -c \"$command\"; else IFS= read -r request || exit 65; printf '%s\\n' '{obsolete_response}'; fi ;;\n  *__ZETA_REMOTE_PLATFORM__*) printf '%s\\n' '__ZETA_REMOTE_PLATFORM__:linux:x86_64:gnu' ;;\n  *__ZETA_REMOTE_RUNTIME_INSTALLED__*) cat >/dev/null; printf '%s\\n' install >> '{installed}'; printf '%s\\n' '__ZETA_REMOTE_RUNTIME_INSTALLED__:{artifact_sha256}:{receipt_runtime}' ;;\n  *__ZETA_REMOTE_RUNTIME_FOUND__*) if [ -f '{installed}' ]; then printf '%s\\n' '__ZETA_REMOTE_RUNTIME_FOUND__:{actual_runtime}'; else {initial_probe}; fi ;;\n  *) exit 64 ;;\nesac\n",
            installed = installed_state.display(),
            profile = profile_root.display(),
            actual_runtime = env!("CARGO_BIN_EXE_zeta"),
        ),
    )
    .unwrap();
    make_executable(path);
}

struct TestRuntimeArtifact {
    archive_size: u64,
    unpacked_size: u64,
    sha256: String,
}

fn create_runtime_archive(directory: &Path) -> TestRuntimeArtifact {
    let path = directory.join("runtime.tar.gz");
    let encoder = GzEncoder::new(File::create(&path).unwrap(), Compression::default());
    let mut builder = Builder::new(encoder);
    let metadata = serde_json::to_vec(&json!({
        "layoutVersion": 2,
        "version": "0.1.0",
        "target": "x86_64-unknown-linux-gnu",
        "entrypoint": "bin/zeta-server",
        "pathDir": "zeta-path",
        "resourcesDir": "zeta-resources",
        "javascriptRuntime": { "kind": "packagedNode" },
        "components": {},
    }))
    .unwrap();
    let mut unpacked_size =
        append_archive_file(&mut builder, "zeta-package.json", &metadata, 0o644);
    unpacked_size += append_archive_file(&mut builder, "bin/zeta-server", b"zeta", 0o755);
    unpacked_size += append_archive_file(&mut builder, "zeta-path/rg", b"rg", 0o755);
    unpacked_size +=
        append_archive_file(&mut builder, "zeta-resources/node/bin/node", b"node", 0o755);
    builder.into_inner().unwrap().finish().unwrap();
    let bytes = fs::read(path).unwrap();
    TestRuntimeArtifact {
        archive_size: bytes.len() as u64,
        unpacked_size,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}

fn append_archive_file(
    builder: &mut Builder<GzEncoder<File>>,
    path: &str,
    bytes: &[u8],
    mode: u32,
) -> u64 {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(bytes.len() as u64);
    header.set_mode(mode);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes).unwrap();
    bytes.len() as u64
}

fn initialize_response(server_schema_hash: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "serverInfo": { "name": "fake-remote", "version": "1" },
            "schemaHash": server_schema_hash,
            "capabilities": {
                "agentInteractions": false,
                "documentCollaboration": false,
                "sessions": false,
                "threads": false,
                "turns": false,
                "resources": false,
                "attachments": false,
                "fileSystem": false,
                "git": false,
                "workspaceSearch": false,
                "codeIndex": false,
                "cloudCodeIndex": false,
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
                "mcpOAuth": false
            },
            "slashCommands": []
        }
    })
    .to_string()
}

fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn test_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-cli-remote-connect-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ))
}
