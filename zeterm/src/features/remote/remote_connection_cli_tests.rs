use tempfile::TempDir;
use zeta_remote_connections::RemoteConnectionCatalog;

use crate::launch::RemoteRuntimeSource;
use crate::launch::ZetermLaunch;
use crate::remote_connection_cli::RemoteConnectionCommandParseError;
use crate::remote_connection_cli::ZetermInvocation;
use crate::remote_connection_cli::ZetermInvocationParseError;

#[test]
fn non_remote_commands_preserve_the_existing_launch_surface() {
    let invocation = ZetermInvocation::parse([
        "--remote".into(),
        "build.example".into(),
        "--workspace".into(),
        "/srv/project".into(),
    ])
    .unwrap();

    assert!(matches!(invocation, ZetermInvocation::Launch(_)));
}

#[test]
fn save_list_replace_remove_and_connect_form_one_credential_free_workflow() {
    let directory = TempDir::new().unwrap();
    let catalog = catalog(&directory);
    let mut output = Vec::new();

    let save = ZetermInvocation::parse([
        "remote".into(),
        "save".into(),
        "Build".into(),
        "--host".into(),
        "build.example".into(),
        "--workspace".into(),
        "/srv/project".into(),
    ])
    .unwrap();
    assert!(
        save.resolve_with_catalog(&catalog, &mut output)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        String::from_utf8(output.clone()).unwrap(),
        "saved\tbuild\tbuild.example\t/srv/project\n"
    );

    output.clear();
    let list = ZetermInvocation::parse(["remote".into(), "list".into()]).unwrap();
    assert!(
        list.resolve_with_catalog(&catalog, &mut output)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        String::from_utf8(output.clone()).unwrap(),
        "build\tbuild.example\t/srv/project\n"
    );

    let duplicate = ZetermInvocation::parse([
        "remote".into(),
        "save".into(),
        "build".into(),
        "--host".into(),
        "other.example".into(),
        "--workspace".into(),
        "/srv/other".into(),
    ])
    .unwrap();
    assert!(
        duplicate
            .resolve_with_catalog(&catalog, &mut output)
            .unwrap_err()
            .contains("already exists")
    );

    let replace = ZetermInvocation::parse([
        "remote".into(),
        "save".into(),
        "build".into(),
        "--host".into(),
        "other.example".into(),
        "--workspace".into(),
        "/srv/other".into(),
        "--replace".into(),
    ])
    .unwrap();
    replace.resolve_with_catalog(&catalog, &mut output).unwrap();

    let connect = ZetermInvocation::parse([
        "remote".into(),
        "connect".into(),
        "build".into(),
        "--runtime".into(),
        "/opt/zeta/bin/zeta-server".into(),
        "--ssh".into(),
        "/usr/bin/ssh".into(),
    ])
    .unwrap();
    let launch = connect
        .resolve_with_catalog(&catalog, &mut output)
        .unwrap()
        .unwrap();
    let ZetermLaunch::Remote {
        profile,
        ssh_executable,
        runtime_source,
    } = launch
    else {
        panic!("expected Remote launch");
    };
    assert_eq!(profile.target().host().as_str(), "other.example");
    assert_eq!(profile.target().workspace().as_str(), "/srv/other");
    assert_eq!(profile.runtime().executable(), "/opt/zeta/bin/zeta-server");
    assert_eq!(ssh_executable.unwrap().to_string_lossy(), "/usr/bin/ssh");
    assert_eq!(runtime_source, RemoteRuntimeSource::ExplicitRuntime);

    let remove =
        ZetermInvocation::parse(["remote".into(), "remove".into(), "build".into()]).unwrap();
    remove.resolve_with_catalog(&catalog, &mut output).unwrap();
    assert!(catalog.connections().unwrap().is_empty());
}

#[test]
fn named_connections_cannot_be_overridden_by_raw_target_flags() {
    let error = ZetermInvocation::parse([
        "remote".into(),
        "connect".into(),
        "build".into(),
        "--remote".into(),
        "other".into(),
        "--workspace".into(),
        "/srv/other".into(),
    ])
    .err()
    .unwrap();

    assert_eq!(
        error,
        ZetermInvocationParseError::Remote(RemoteConnectionCommandParseError::NamedTargetConflict)
    );
}

#[test]
fn save_requires_a_valid_name_host_and_absolute_workspace() {
    let invalid_name = ZetermInvocation::parse([
        "remote".into(),
        "save".into(),
        "build server".into(),
        "--host".into(),
        "build".into(),
        "--workspace".into(),
        "/srv/project".into(),
    ]);
    assert!(matches!(
        invalid_name,
        Err(ZetermInvocationParseError::Remote(
            RemoteConnectionCommandParseError::Name(_)
        ))
    ));

    let missing_host = ZetermInvocation::parse([
        "remote".into(),
        "save".into(),
        "build".into(),
        "--workspace".into(),
        "/srv/project".into(),
    ]);
    assert!(matches!(
        missing_host,
        Err(ZetermInvocationParseError::Remote(
            RemoteConnectionCommandParseError::RequiredOption {
                command: "save",
                flag: "--host",
            }
        ))
    ));

    let relative_workspace = ZetermInvocation::parse([
        "remote".into(),
        "save".into(),
        "build".into(),
        "--host".into(),
        "build".into(),
        "--workspace".into(),
        "project".into(),
    ]);
    assert!(matches!(
        relative_workspace,
        Err(ZetermInvocationParseError::Remote(
            RemoteConnectionCommandParseError::Address(_)
        ))
    ));
}

fn catalog(directory: &TempDir) -> RemoteConnectionCatalog {
    RemoteConnectionCatalog::new(directory.path().join("targets.json"))
}
