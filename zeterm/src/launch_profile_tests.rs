use crate::launch::LaunchParseError;
use crate::launch::RemoteRuntimeCatalogSource;
use crate::launch::RemoteRuntimeSource;
use crate::launch::ZetermLaunch;

use zeta_remote::RemoteProfile;
use zeta_remote::RemoteRuntime;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;
use zeta_remote_connections::RemoteConnectionProfileStore;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::Path;

#[cfg(unix)]
use crate::launch_test_support::initialize_response;
#[cfg(unix)]
use crate::launch_test_support::make_executable;
#[cfg(unix)]
use zeta_app_server_protocol::schema_hash;

#[test]
fn runtime_selection_flags_are_complete_and_unambiguous() {
    assert_eq!(
        ZetermLaunch::parse([
            "--remote".into(),
            "build".into(),
            "--workspace".into(),
            "/srv/project".into(),
            "--runtime-catalog".into(),
            "/opt/zeterm/catalog.json".into(),
        ]),
        Err(LaunchParseError::IncompleteRuntimeCatalog)
    );
    assert_eq!(
        ZetermLaunch::parse([
            "--remote".into(),
            "build".into(),
            "--workspace".into(),
            "/srv/project".into(),
            "--runtime".into(),
            "/opt/zeta/bin/zeta".into(),
            "--runtime-catalog".into(),
            "/opt/zeterm/catalog.json".into(),
            "--runtime-catalog-sha256".into(),
            "0".repeat(64),
        ]),
        Err(LaunchParseError::RuntimeCatalogConflictsWithRuntime)
    );
    assert_eq!(
        ZetermLaunch::parse([
            "--remote".into(),
            "build".into(),
            "--workspace".into(),
            "/srv/project".into(),
            "--runtime-catalog".into(),
            "/opt/zeterm/catalog.json".into(),
            "--runtime-catalog-url".into(),
            "https://releases.example/zeta/catalog.json".into(),
            "--runtime-catalog-sha256".into(),
            "0".repeat(64),
        ]),
        Err(LaunchParseError::IncompleteRuntimeCatalog)
    );

    let rollback = ZetermLaunch::parse([
        "--remote".into(),
        "build".into(),
        "--workspace".into(),
        "/srv/project".into(),
        "--rollback-runtime".into(),
    ])
    .unwrap();
    assert!(matches!(
        rollback,
        ZetermLaunch::Remote {
            runtime_source: RemoteRuntimeSource::StoredRollback,
            ..
        }
    ));
    assert_eq!(
        ZetermLaunch::parse([
            "--remote".into(),
            "build".into(),
            "--workspace".into(),
            "/srv/project".into(),
            "--rollback-runtime".into(),
            "--runtime".into(),
            "/runtime/one/bin/zeta".into(),
        ]),
        Err(LaunchParseError::RollbackRuntimeConflictsWithSelection)
    );
}

#[test]
fn network_runtime_catalog_requires_an_authenticated_release_and_absolute_cache() {
    let launch = ZetermLaunch::parse([
        "--remote".into(),
        "build".into(),
        "--workspace".into(),
        "/srv/project".into(),
        "--runtime-catalog-url".into(),
        "https://releases.example/zeta/catalog.json".into(),
        "--runtime-catalog-sha256".into(),
        "a".repeat(64),
        "--runtime-cache".into(),
        "/var/tmp/zeterm-runtime-cache".into(),
    ])
    .unwrap();
    let ZetermLaunch::Remote {
        runtime_source:
            RemoteRuntimeSource::DefaultRuntime {
                catalog: Some(RemoteRuntimeCatalogSource::Network { release, cache }),
            },
        ..
    } = launch
    else {
        panic!("expected network runtime catalog");
    };
    assert_eq!(
        release.catalog_url(),
        "https://releases.example/zeta/catalog.json"
    );
    assert_eq!(release.expected_sha256(), "a".repeat(64));
    assert_eq!(cache.root(), Path::new("/var/tmp/zeterm-runtime-cache"));

    assert!(matches!(
        ZetermLaunch::parse([
            "--remote".into(),
            "build".into(),
            "--workspace".into(),
            "/srv/project".into(),
            "--runtime-catalog-url".into(),
            "http://releases.example/zeta/catalog.json".into(),
            "--runtime-catalog-sha256".into(),
            "a".repeat(64),
        ]),
        Err(LaunchParseError::InvalidRuntimeCatalog(_))
    ));
    assert!(matches!(
        ZetermLaunch::parse([
            "--remote".into(),
            "build".into(),
            "--workspace".into(),
            "/srv/project".into(),
            "--runtime-catalog-url".into(),
            "https://releases.example/zeta/catalog.json".into(),
            "--runtime-catalog-sha256".into(),
            "a".repeat(64),
            "--runtime-cache".into(),
            "relative".into(),
        ]),
        Err(LaunchParseError::InvalidRuntimeCatalog(_))
    ));
}

#[cfg(unix)]
#[test]
fn rollback_is_compatibility_checked_before_the_stored_generations_swap() {
    let directory = tempfile::tempdir().unwrap();
    let store = profile_store(&directory);
    let target = target();
    let previous = profile(target.clone(), "/runtime/one/bin/zeta");
    let active = profile(target.clone(), "/runtime/two/bin/zeta");
    store.activate(&previous).unwrap();
    store.activate(&active).unwrap();
    let fake_ssh = directory.path().join("fake-ssh");
    let log = directory.path().join("ssh.log");
    write_runtime_fake_ssh(&fake_ssh, &log, "/runtime/one/bin/zeta", "obsolete-schema");

    let mut rejected = rollback_launch(&fake_ssh);
    let error = rejected
        .prepare_remote_runtime_with_store(&store)
        .unwrap_err();
    assert!(error.contains("readiness or compatibility check failed"));
    assert_eq!(
        store.connection(&target).unwrap().unwrap().active_profile(),
        active
    );

    write_runtime_fake_ssh(&fake_ssh, &log, "/runtime/one/bin/zeta", &schema_hash());
    let mut accepted = rollback_launch(&fake_ssh);
    accepted.prepare_remote_runtime_with_store(&store).unwrap();

    let ZetermLaunch::Remote { profile, .. } = accepted else {
        panic!("expected Remote launch");
    };
    assert_eq!(profile.runtime(), previous.runtime());
    let stored = store.connection(&target).unwrap().unwrap();
    assert_eq!(stored.active_runtime(), previous.runtime());
    assert_eq!(stored.previous_runtime(), Some(active.runtime()));
    let commands = fs::read_to_string(log).unwrap();
    assert!(commands.contains("/runtime/one/bin/zeta"));
    assert!(!commands.contains("/runtime/two/bin/zeta"));
}

#[cfg(unix)]
#[test]
fn corrupt_profile_store_fails_before_ssh_is_started() {
    let directory = tempfile::tempdir().unwrap();
    let store = profile_store(&directory);
    fs::write(store.path(), b"not-json").unwrap();
    let marker = directory.path().join("ssh-started");
    let fake_ssh = directory.path().join("fake-ssh");
    fs::write(
        &fake_ssh,
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexit 90\n",
            marker.display()
        ),
    )
    .unwrap();
    make_executable(&fake_ssh);
    let mut launch = default_launch(&fake_ssh);

    let error = launch
        .prepare_remote_runtime_with_store(&store)
        .unwrap_err();

    assert!(error.contains("invalid JSON"));
    assert!(!marker.exists());
}

#[cfg(unix)]
fn rollback_launch(fake_ssh: &Path) -> ZetermLaunch {
    let mut arguments = remote_arguments(fake_ssh);
    arguments.push("--rollback-runtime".into());
    ZetermLaunch::parse(arguments).unwrap()
}

#[cfg(unix)]
fn default_launch(fake_ssh: &Path) -> ZetermLaunch {
    ZetermLaunch::parse(remote_arguments(fake_ssh)).unwrap()
}

#[cfg(unix)]
fn remote_arguments(fake_ssh: &Path) -> Vec<String> {
    vec![
        "--remote".into(),
        "build".into(),
        "--workspace".into(),
        "/srv/project".into(),
        "--ssh".into(),
        fake_ssh.to_string_lossy().into_owned(),
    ]
}

#[cfg(unix)]
fn write_runtime_fake_ssh(path: &Path, log: &Path, runtime: &str, server_schema_hash: &str) {
    let response = initialize_response(server_schema_hash);
    fs::write(
        path,
        format!(
            "#!/bin/sh\ncommand=''\nfor argument in \"$@\"; do command=$argument; done\nprintf '%s\\n' \"$command\" >> '{}'\ncase \"$command\" in\n  *\"'remote-server' 'connect'\"*) IFS= read -r request || exit 65; printf '%s\\n' '{}' ;;\n  *) printf '%s\\n' '__ZETA_REMOTE_RUNTIME_FOUND__:{}' ;;\nesac\n",
            log.display(),
            response,
            runtime,
        ),
    )
    .unwrap();
    make_executable(path);
}

fn profile_store(directory: &tempfile::TempDir) -> RemoteConnectionProfileStore {
    RemoteConnectionProfileStore::new(directory.path().join("remote-connections.json"))
}

fn target() -> SshTarget {
    SshTarget::new(
        SshHost::parse("build").unwrap(),
        RemoteWorkspacePath::parse("/srv/project").unwrap(),
    )
}

fn profile(target: SshTarget, runtime: &str) -> RemoteProfile {
    RemoteProfile::new(target, RemoteRuntime::new(runtime).unwrap())
}
