use std::fs;
use std::path::{Path, PathBuf};

use super::*;

struct FixedCandidates(Vec<PathBuf>);

impl LanguageServerExecutableCandidates for FixedCandidates {
    fn candidates(
        &self,
        executable_name: &str,
    ) -> Result<Vec<PathBuf>, LanguageServerCatalogError> {
        Ok(self
            .0
            .iter()
            .filter(|path| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == executable_name)
            })
            .cloned()
            .collect())
    }
}

#[test]
fn json_and_shell_builtins_resolve_with_canonical_routes_and_launch_arguments() {
    let directory = tempfile::tempdir().expect("directory");
    let executables = [
        RUST_ANALYZER_SERVER_ID,
        JSON_LANGUAGE_SERVER_ID,
        BASH_LANGUAGE_SERVER_ID,
    ]
    .map(|name| {
        let path = directory.path().join(platform_executable_name(name));
        fs::write(&path, b"test server").expect("executable");
        make_executable(&path);
        path
    });

    let resolution = LanguageServerCatalog::default()
        .resolve(
            &FixedCandidates(executables.into()),
            LanguageServerExecutionPolicy::Allowed,
            directory.path(),
        )
        .expect("resolution");

    assert_eq!(resolution.definitions().len(), 3);
    let routes = resolution
        .definitions()
        .iter()
        .map(|definition| {
            (
                definition.name().as_str(),
                definition.language_ids().collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        routes,
        vec![
            (RUST_ANALYZER_SERVER_ID, vec!["rust"]),
            (JSON_LANGUAGE_SERVER_ID, vec!["json", "jsonc"]),
            (BASH_LANGUAGE_SERVER_ID, vec!["shellscript"]),
        ]
    );
}

#[test]
fn automatic_rust_analyzer_resolves_from_the_frozen_host_path() {
    let directory = tempfile::tempdir().expect("directory");
    let executable = directory.path().join(executable_name());
    fs::write(&executable, b"test server").expect("executable");
    make_executable(&executable);
    let candidates = FixedCandidates(vec![executable.clone()]);

    let resolution = LanguageServerCatalog::default()
        .resolve(
            &candidates,
            LanguageServerExecutionPolicy::Allowed,
            directory.path(),
        )
        .expect("resolution");

    assert_eq!(resolution.definitions().len(), 1);
    assert!(matches!(
        resolution.entries()[0].state(),
        LanguageServerCatalogState::Resolved { executable: resolved }
            if resolved == &fs::canonicalize(executable).unwrap()
    ));
}

#[test]
fn execution_policy_blocks_resolution_before_inspecting_candidates() {
    let candidates = FixedCandidates(vec![PathBuf::from("/does/not/exist")]);
    let resolution = LanguageServerCatalog::default()
        .resolve(
            &candidates,
            LanguageServerExecutionPolicy::Disallowed,
            Path::new("/workspace"),
        )
        .expect("resolution");

    assert!(resolution.definitions().is_empty());
    assert_eq!(
        resolution.entries()[0].state(),
        &LanguageServerCatalogState::ExecutionDisallowed
    );
}

#[test]
fn invalid_explicit_executable_is_authoritative_and_does_not_fall_back() {
    let directory = tempfile::tempdir().expect("directory");
    let discovered = directory.path().join(executable_name());
    fs::write(&discovered, b"test server").expect("executable");
    make_executable(&discovered);
    let candidates = FixedCandidates(vec![discovered]);
    let catalog = LanguageServerCatalog::new(
        LanguageServerPreference::enabled()
            .with_explicit_executable(directory.path().join("missing")),
    );

    let resolution = catalog
        .resolve(
            &candidates,
            LanguageServerExecutionPolicy::Allowed,
            directory.path(),
        )
        .expect("resolution");

    assert!(resolution.definitions().is_empty());
    assert_eq!(
        resolution.entries()[0].state(),
        &LanguageServerCatalogState::ExecutableUnavailable
    );
}

fn executable_name() -> &'static str {
    if cfg!(windows) {
        "rust-analyzer.exe"
    } else {
        "rust-analyzer"
    }
}

fn platform_executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("permissions");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
